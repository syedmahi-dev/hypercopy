use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, FILETIME, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetFileTime, ReadFile, SetEndOfFile, SetFilePointerEx, SetFileTime,
    WriteFile, CREATE_ALWAYS, FILE_BEGIN, FILE_FLAG_NO_BUFFERING,
    FILE_FLAG_SEQUENTIAL_SCAN, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use crate::ui::ProgressStats;
use crate::utils::aligned_buf::AlignedBuffer;

pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;
const SECTOR_SIZE: usize = 4096;
const CHUNK_SIZE: usize = 8 * 1024 * 1024; // 8 MB chunk per worker stream

/// Parallel Multi-Stream Segmented File Copy Engine.
/// Slices a single large file into N parallel independent streams (like Internet Download Manager / aria2),
/// copying all segments concurrently using sector-aligned Bare-Metal Direct I/O (DMA).
/// Saturates multi-channel NVMe flash controllers and deep PCIe submission queues.
pub fn parallel_file_copy(
    _src: &Path,
    dst: &Path,
    src_wide: &[u16],
    dst_wide: &[u16],
    file_size: u64,
    num_streams: usize,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    if file_size == 0 {
        std::fs::File::create(dst).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let streams = num_streams.clamp(2, 32);

    unsafe {
        // Step 1: Pre-allocate the entire destination file to eliminate NTFS allocation pauses
        let dst_pre = CreateFileW(
            dst_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            CREATE_ALWAYS,
            FILE_FLAG_NO_BUFFERING,
            std::ptr::null_mut(),
        );

        if dst_pre == INVALID_HANDLE_VALUE {
            return Err(format!("ParallelCopy: Failed to pre-allocate destination (error {})", GetLastError()));
        }

        let mut pos: i64 = 0;
        let aligned_alloc = ((file_size as usize + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1)) as i64;
        SetFilePointerEx(dst_pre, aligned_alloc, &mut pos, FILE_BEGIN);
        SetEndOfFile(dst_pre);
        CloseHandle(dst_pre);

        // Step 2: Calculate segmented byte ranges (aligned to SECTOR_SIZE)
        let raw_seg_size = (file_size + streams as u64 - 1) / streams as u64;
        let seg_size = (raw_seg_size + SECTOR_SIZE as u64 - 1) & !(SECTOR_SIZE as u64 - 1);

        let mut segments = Vec::new();
        for i in 0..streams {
            let start = i as u64 * seg_size;
            if start >= file_size {
                break;
            }
            let end = ((i as u64 + 1) * seg_size).min(file_size);
            segments.push((start, end));
        }

        // Step 3: Dispatch parallel streams across Rayon thread pool
        let src_wide_arc = Arc::new(src_wide.to_vec());
        let dst_wide_arc = Arc::new(dst_wide.to_vec());
        let stats_clone = Arc::clone(stats);

        let results: Vec<Result<(), String>> = rayon::scope(|s| {
            let (tx, rx) = std::sync::mpsc::channel();

            for (stream_id, &(start_offset, end_offset)) in segments.iter().enumerate() {
                let tx = tx.clone();
                let src_w = Arc::clone(&src_wide_arc);
                let dst_w = Arc::clone(&dst_wide_arc);
                let st = Arc::clone(&stats_clone);

                s.spawn(move |_| {
                    let res = copy_segment(
                        &src_w,
                        &dst_w,
                        start_offset,
                        end_offset,
                        stream_id,
                        &st,
                    );
                    let _ = tx.send(res);
                });
            }

            drop(tx);
            rx.into_iter().collect()
        });

        // Check if any stream encountered an error
        for r in results {
            r?;
        }

        // Step 4: Truncate destination to exact byte length and restore timestamps
        let normal_src = CreateFileW(
            src_wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );

        let mut create_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut access_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let mut write_time = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };

        if normal_src != INVALID_HANDLE_VALUE {
            GetFileTime(normal_src, &mut create_time, &mut access_time, &mut write_time);
            CloseHandle(normal_src);
        }

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

/// Worker thread function copying a single byte range using sector-aligned Direct DMA I/O.
unsafe fn copy_segment(
    src_wide: &[u16],
    dst_wide: &[u16],
    start_offset: u64,
    end_offset: u64,
    stream_id: usize,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    let seg_bytes = end_offset - start_offset;
    if seg_bytes == 0 {
        return Ok(());
    }

    unsafe {
        // Open independent unbuffered handle for reading this segment
        let src_h = CreateFileW(
            src_wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN,
            std::ptr::null_mut(),
        );

        if src_h == INVALID_HANDLE_VALUE {
            return Err(format!("Stream {}: Failed to open source (error {})", stream_id, GetLastError()));
        }

        // Open independent unbuffered handle for writing this segment
        let dst_h = CreateFileW(
            dst_wide.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_NO_BUFFERING,
            std::ptr::null_mut(),
        );

        if dst_h == INVALID_HANDLE_VALUE {
            CloseHandle(src_h);
            return Err(format!("Stream {}: Failed to open destination (error {})", stream_id, GetLastError()));
        }

        // Seek both handles to start_offset
        let mut pos: i64 = 0;
        SetFilePointerEx(src_h, start_offset as i64, &mut pos, FILE_BEGIN);
        SetFilePointerEx(dst_h, start_offset as i64, &mut pos, FILE_BEGIN);

        // Allocate sector-aligned buffer for this stream
        let mut buffer = match AlignedBuffer::new(CHUNK_SIZE, SECTOR_SIZE) {
            Ok(b) => b,
            Err(e) => {
                CloseHandle(src_h);
                CloseHandle(dst_h);
                return Err(e);
            }
        };

        let mut transferred = 0u64;
        let mut err: Option<String> = None;

        while transferred < seg_bytes {
            let remaining = (seg_bytes - transferred) as usize;
            let chunk = remaining.min(buffer.capacity());
            let aligned_chunk = (chunk + SECTOR_SIZE - 1) & !(SECTOR_SIZE - 1);

            let mut bytes_read: u32 = 0;
            let r_ok = ReadFile(
                src_h,
                buffer.as_mut_ptr(),
                aligned_chunk as u32,
                &mut bytes_read,
                std::ptr::null_mut(),
            );

            if r_ok == 0 {
                err = Some(format!("Stream {}: ReadFile error {}", stream_id, GetLastError()));
                break;
            }

            let mut bytes_written: u32 = 0;
            let w_ok = WriteFile(
                dst_h,
                buffer.as_ptr(),
                aligned_chunk as u32,
                &mut bytes_written,
                std::ptr::null_mut(),
            );

            if w_ok == 0 {
                err = Some(format!("Stream {}: WriteFile error {}", stream_id, GetLastError()));
                break;
            }

            transferred += chunk as u64;
            stats.copied_bytes.fetch_add(chunk as u64, Ordering::Relaxed);
        }

        CloseHandle(src_h);
        CloseHandle(dst_h);

        if let Some(e) = err {
            Err(e)
        } else {
            Ok(())
        }
    }
}
