use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE, FILETIME};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileTime, ReadFile, SetEndOfFile, SetFilePointerEx, SetFileTime,
    WriteFile, CREATE_ALWAYS, FILE_BEGIN, FILE_FLAG_NO_BUFFERING,
    FILE_FLAG_SEQUENTIAL_SCAN, FILE_FLAG_WRITE_THROUGH, FILE_SHARE_READ, OPEN_EXISTING,
};
use crate::ui::ProgressStats;
use crate::utils::aligned_buf::AlignedBuffer;
use crate::utils::path_to_wide;

pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;
const SECTOR_SIZE: usize = 4096;

/// Unbuffered Bare-Metal Direct I/O (DMA) Copy Engine.
/// Reads directly from source NVMe/SSD into 4096-byte aligned RAM buffers and writes directly
/// to destination NVMe/SSD without passing through the OS Page Cache.
/// Eliminates cache thrashing and memory double-copy overhead for multi-gigabyte transfers.
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
        // Open source file unbuffered
        let src_handle = CreateFileW(
            src_wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN,
            std::ptr::null_mut(),
        );

        if src_handle == INVALID_HANDLE_VALUE {
            return Err(format!("DirectIO: Failed to open source unbuffered (error {})", GetLastError()));
        }

        // Open destination file unbuffered
        let dst_handle = CreateFileW(
            dst_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            CREATE_ALWAYS,
            FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH,
            std::ptr::null_mut(),
        );

        if dst_handle == INVALID_HANDLE_VALUE {
            CloseHandle(src_handle);
            return Err(format!("DirectIO: Failed to create destination unbuffered (error {})", GetLastError()));
        }

        // Pre-allocate destination file space to avoid disk fragmentation
        let mut new_pos: i64 = 0;
        let aligned_alloc = ((file_size as usize + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1)) as i64;
        SetFilePointerEx(dst_handle, aligned_alloc, &mut new_pos, FILE_BEGIN);
        SetEndOfFile(dst_handle);
        // Rewind pointer to beginning
        SetFilePointerEx(dst_handle, 0, &mut new_pos, FILE_BEGIN);

        // Allocate sector-aligned memory buffer
        let buf_size = buffer_size.max(SECTOR_SIZE);
        let mut buffer = match AlignedBuffer::new(buf_size, SECTOR_SIZE) {
            Ok(b) => b,
            Err(e) => {
                CloseHandle(src_handle);
                CloseHandle(dst_handle);
                return Err(e);
            }
        };

        let mut total_transferred: u64 = 0;
        let mut transfer_err: Option<String> = None;

        while total_transferred < file_size {
            let remaining = (file_size - total_transferred) as usize;
            let chunk = remaining.min(buffer.capacity());
            // Align request length up to sector boundary for unbuffered I/O
            let aligned_len = (chunk + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1);

            let mut bytes_read: u32 = 0;
            let read_ok = ReadFile(
                src_handle,
                buffer.as_mut_ptr(),
                aligned_len as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            );

            if read_ok == 0 {
                transfer_err = Some(format!("DirectIO: ReadFile error {}", GetLastError()));
                break;
            }

            let mut bytes_written: u32 = 0;
            let write_ok = WriteFile(
                dst_handle,
                buffer.as_ptr(),
                aligned_len as u32,
                &mut bytes_written,
                std::ptr::null_mut(),
            );

            if write_ok == 0 {
                transfer_err = Some(format!("DirectIO: WriteFile error {}", GetLastError()));
                break;
            }

            total_transferred += chunk as u64;
            stats.copied_bytes.fetch_add(chunk as u64, Ordering::Relaxed);
        }

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
