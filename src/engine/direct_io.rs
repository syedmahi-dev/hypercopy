use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileTime, ReadFile, SetEndOfFile, SetFilePointerEx, SetFileTime,
    WriteFile, CREATE_ALWAYS, FILE_BEGIN, FILE_FLAG_NO_BUFFERING,
    FILE_FLAG_OVERLAPPED, FILE_FLAG_SEQUENTIAL_SCAN, FILE_FLAG_WRITE_THROUGH,
    FILE_SHARE_READ, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent};
use crate::ui::ProgressStats;
use crate::utils::aligned_buf::AlignedBuffer;
use crate::utils::path_to_wide;

pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;
const SECTOR_SIZE: usize = 4096;
const ERROR_IO_PENDING: u32 = 997;

unsafe fn make_overlapped(offset: u64, event: HANDLE) -> OVERLAPPED {
    unsafe {
        let mut ov: OVERLAPPED = std::mem::zeroed();
        ov.Anonymous.Anonymous.Offset = offset as u32;
        ov.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        ov.hEvent = event;
        ov
    }
}

/// Unbuffered Bare-Metal Direct I/O (DMA) Copy Engine with Overlapped Double-Buffering.
/// Uses dual sector-aligned 4096-byte memory buffers and Windows OVERLAPPED asynchronous I/O
/// to overlap PCIe NVMe Read and Write channels simultaneously.
/// Bypasses the Windows Page Cache entirely, eliminating CPU cache thrashing.
pub fn direct_io_copy(
    src: &Path,
    dst: &Path,
    file_size: u64,
    buffer_size: usize,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    if file_size == 0 {
        std::fs::File::create(dst).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let src_wide = path_to_wide(src);
    let dst_wide = path_to_wide(dst);

    unsafe {
        // Open source file unbuffered with overlapped I/O support
        let src_handle = CreateFileW(
            src_wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN | FILE_FLAG_OVERLAPPED,
            std::ptr::null_mut(),
        );

        if src_handle == INVALID_HANDLE_VALUE {
            return Err(format!("DirectIO: Failed to open source unbuffered (error {})", GetLastError()));
        }

        // Open destination file unbuffered with overlapped I/O support
        let dst_handle = CreateFileW(
            dst_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            CREATE_ALWAYS,
            FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH | FILE_FLAG_OVERLAPPED,
            std::ptr::null_mut(),
        );

        if dst_handle == INVALID_HANDLE_VALUE {
            CloseHandle(src_handle);
            return Err(format!("DirectIO: Failed to create destination unbuffered (error {})", GetLastError()));
        }

        // Pre-allocate destination file space to avoid filesystem fragmentation
        let mut new_pos: i64 = 0;
        let aligned_alloc = ((file_size as usize + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1)) as i64;
        SetFilePointerEx(dst_handle, aligned_alloc, &mut new_pos, FILE_BEGIN);
        SetEndOfFile(dst_handle);
        SetFilePointerEx(dst_handle, 0, &mut new_pos, FILE_BEGIN);

        // Allocate dual sector-aligned memory buffers for double-buffering
        let buf_size = buffer_size.max(SECTOR_SIZE);
        let mut buf_a = match AlignedBuffer::new(buf_size, SECTOR_SIZE) {
            Ok(b) => b,
            Err(e) => {
                CloseHandle(src_handle);
                CloseHandle(dst_handle);
                return Err(e);
            }
        };

        let mut buf_b = match AlignedBuffer::new(buf_size, SECTOR_SIZE) {
            Ok(b) => b,
            Err(e) => {
                CloseHandle(src_handle);
                CloseHandle(dst_handle);
                return Err(e);
            }
        };

        // Create synchronization events for asynchronous I/O completion
        let read_event = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
        let write_event = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());

        if read_event.is_null() || write_event.is_null() {
            if !read_event.is_null() { CloseHandle(read_event); }
            if !write_event.is_null() { CloseHandle(write_event); }
            CloseHandle(src_handle);
            CloseHandle(dst_handle);
            return Err("DirectIO: Failed to create overlapped events".to_string());
        }

        let mut read_offset: u64 = 0;
        let mut write_offset: u64 = 0;
        let mut transfer_err: Option<String> = None;

        // --- Step 1: Prime the pipeline by reading chunk 0 into buf_a ---
        let first_chunk = ((file_size - read_offset) as usize).min(buf_a.capacity());
        let first_aligned = (first_chunk + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1);

        let mut ov_read = make_overlapped(read_offset, read_event);
        ResetEvent(read_event);

        let r_ok = ReadFile(
            src_handle,
            buf_a.as_mut_ptr(),
            first_aligned as u32,
            std::ptr::null_mut(),
            &mut ov_read,
        );

        if r_ok == 0 && GetLastError() != ERROR_IO_PENDING {
            transfer_err = Some(format!("DirectIO: Initial ReadFile error {}", GetLastError()));
        } else {
            let mut bytes_transferred: u32 = 0;
            if GetOverlappedResult(src_handle, &ov_read, &mut bytes_transferred, 1) == 0 {
                transfer_err = Some(format!("DirectIO: Initial GetOverlappedResult read error {}", GetLastError()));
            }
        }

        read_offset += first_chunk as u64;
        let mut current_valid_len = first_chunk;
        let mut current_aligned_len = first_aligned;

        // --- Step 2: Overlapped double-buffering loop ---
        // While buf_a is written to dst, buf_b is concurrently read from src.
        while transfer_err.is_none() && write_offset < file_size {
            // Start asynchronous write of buf_a
            let mut ov_write = make_overlapped(write_offset, write_event);
            ResetEvent(write_event);

            let w_ok = WriteFile(
                dst_handle,
                buf_a.as_ptr(),
                current_aligned_len as u32,
                std::ptr::null_mut(),
                &mut ov_write,
            );

            if w_ok == 0 && GetLastError() != ERROR_IO_PENDING {
                transfer_err = Some(format!("DirectIO: WriteFile error {}", GetLastError()));
                break;
            }

            // Concurrently start asynchronous read into buf_b if more data remains
            let mut next_valid_len = 0usize;
            let mut next_aligned_len = 0usize;
            let has_next_read = read_offset < file_size;

            if has_next_read {
                next_valid_len = ((file_size - read_offset) as usize).min(buf_b.capacity());
                next_aligned_len = (next_valid_len + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1);

                ov_read = make_overlapped(read_offset, read_event);
                ResetEvent(read_event);

                let next_r_ok = ReadFile(
                    src_handle,
                    buf_b.as_mut_ptr(),
                    next_aligned_len as u32,
                    std::ptr::null_mut(),
                    &mut ov_read,
                );

                if next_r_ok == 0 && GetLastError() != ERROR_IO_PENDING {
                    transfer_err = Some(format!("DirectIO: Overlapped ReadFile error {}", GetLastError()));
                    break;
                }
            }

            // Wait for the write of buf_a to complete
            let mut write_bytes_done: u32 = 0;
            if GetOverlappedResult(dst_handle, &ov_write, &mut write_bytes_done, 1) == 0 {
                transfer_err = Some(format!("DirectIO: Write GetOverlappedResult error {}", GetLastError()));
                break;
            }

            stats.copied_bytes.fetch_add(current_valid_len as u64, Ordering::Relaxed);
            write_offset += current_valid_len as u64;

            // Wait for the concurrent read of buf_b to complete (if any)
            if has_next_read {
                let mut read_bytes_done: u32 = 0;
                if GetOverlappedResult(src_handle, &ov_read, &mut read_bytes_done, 1) == 0 {
                    transfer_err = Some(format!("DirectIO: Read GetOverlappedResult error {}", GetLastError()));
                    break;
                }
                read_offset += next_valid_len as u64;
            }

            // Swap buffers: buf_b (newly read) becomes buf_a (ready to write)
            std::mem::swap(&mut buf_a, &mut buf_b);
            current_valid_len = next_valid_len;
            current_aligned_len = next_aligned_len;
        }

        CloseHandle(read_event);
        CloseHandle(write_event);

        // Capture source timestamps before closing
        let mut create_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut access_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut write_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        GetFileTime(src_handle, &mut create_time, &mut access_time, &mut write_time);

        CloseHandle(src_handle);
        CloseHandle(dst_handle);

        if let Some(err) = transfer_err {
            return Err(err);
        }

        // Post-processing: Truncate destination to exact file length and restore timestamps
        let normal_dst = CreateFileW(
            dst_wide.as_ptr(),
            GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );

        if normal_dst != INVALID_HANDLE_VALUE {
            let mut pos: i64 = 0;
            SetFilePointerEx(normal_dst, file_size as i64, &mut pos, FILE_BEGIN);
            SetEndOfFile(normal_dst);
            SetFileTime(normal_dst, &create_time, &access_time, &write_time);
            CloseHandle(normal_dst);
        }

        Ok(())
    }
}
