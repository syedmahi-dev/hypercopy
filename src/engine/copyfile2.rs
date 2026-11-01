use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Storage::FileSystem::CopyFileExW;
use crate::ui::ProgressStats;
use crate::utils::path_to_wide;

pub const COPY_FILE_NO_BUFFERING: u32 = 0x00001000;

pub type HRESULT = i32;

/// Execute high-speed copy using Windows CopyFileExW kernel offload API with unbuffered bypass.
pub fn copy_file2(
    src: &Path,
    dst: &Path,
    file_size: u64,
    no_buffering: bool,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    let src_wide = path_to_wide(src);
    let dst_wide = path_to_wide(dst);
    copy_file2_wide(&src_wide, &dst_wide, file_size, no_buffering, stats)
}

/// Copy using pre-computed wide UTF-16 paths, eliminating string encoding conversions.
pub fn copy_file2_wide(
    src_wide: &[u16],
    dst_wide: &[u16],
    file_size: u64,
    no_buffering: bool,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    let copy_flags = if no_buffering {
        COPY_FILE_NO_BUFFERING
    } else {
        0
    };

    let ok = unsafe {
        CopyFileExW(
            src_wide.as_ptr(),
            dst_wide.as_ptr(),
            None,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            copy_flags,
        )
    };

    if ok != 0 {
        stats.copied_bytes.fetch_add(file_size, Ordering::Relaxed);
        Ok(())
    } else {
        let err = unsafe { GetLastError() };
        Err(format!("CopyFileExW failed with error {}", err))
    }
}
