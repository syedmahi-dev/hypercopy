use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, SetEndOfFile, SetFilePointerEx, CREATE_ALWAYS, FILE_BEGIN,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use crate::utils::path_to_wide;

pub const FSCTL_DUPLICATE_EXTENTS_TO_FILE: u32 = 0x00098344;
pub const GENERIC_READ: u32 = 0x80000000;
pub const GENERIC_WRITE: u32 = 0x40000000;

pub static REFLINK_SUPPORTED: AtomicBool = AtomicBool::new(true);

pub fn is_reflink_supported() -> bool {
    REFLINK_SUPPORTED.load(Ordering::Relaxed)
}

#[repr(C)]
struct DuplicateExtentsData {
    file_handle: HANDLE,
    source_file_offset: i64,
    target_file_offset: i64,
    byte_count: i64,
}

/// Attempt instantaneous Copy-On-Write (Reflink / Block Cloning) on ReFS / modern volumes.
/// If supported and on the same volume, this clones files of any size in < 1 millisecond.
pub fn try_reflink_copy(src: &Path, dst: &Path, file_size: u64) -> Result<(), String> {
    if !is_reflink_supported() {
        return Err("Reflink disabled on current filesystem".to_string());
    }

    if file_size == 0 {
        // Empty file - just create it
        std::fs::File::create(dst).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let src_wide = path_to_wide(src);
    let dst_wide = path_to_wide(dst);

    unsafe {
        // Open source with read access
        let src_handle = CreateFileW(
            src_wide.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );

        if src_handle == INVALID_HANDLE_VALUE {
            return Err(format!("Failed to open source file: error {}", GetLastError()));
        }

        // Open/create destination with write & read access
        let dst_handle = CreateFileW(
            dst_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            CREATE_ALWAYS,
            0,
            std::ptr::null_mut(),
        );

        if dst_handle == INVALID_HANDLE_VALUE {
            CloseHandle(src_handle);
            return Err(format!("Failed to open destination file: error {}", GetLastError()));
        }

        // Destination must be pre-allocated to the file size for extent duplication
        let mut new_pos: i64 = 0;
        let set_res = SetFilePointerEx(dst_handle, file_size as i64, &mut new_pos, FILE_BEGIN);
        if set_res == 0 || SetEndOfFile(dst_handle) == 0 {
            CloseHandle(src_handle);
            CloseHandle(dst_handle);
            return Err("Failed to set destination file length".to_string());
        }

        let extents = DuplicateExtentsData {
            file_handle: src_handle,
            source_file_offset: 0,
            target_file_offset: 0,
            byte_count: file_size as i64,
        };

        let mut bytes_returned: u32 = 0;
        let io_res = DeviceIoControl(
            dst_handle,
            FSCTL_DUPLICATE_EXTENTS_TO_FILE,
            &extents as *const _ as *const _,
            std::mem::size_of::<DuplicateExtentsData>() as u32,
            std::ptr::null_mut(),
            0,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        CloseHandle(src_handle);
        CloseHandle(dst_handle);

        if io_res != 0 {
            Ok(())
        } else {
            let err = GetLastError();
            // 1 = ERROR_INVALID_FUNCTION (NTFS doesn't support block cloning)
            // 50 = ERROR_NOT_SUPPORTED
            // 17 = ERROR_NOT_SAME_DEVICE
            if err == 1 || err == 50 || err == 17 {
                REFLINK_SUPPORTED.store(false, Ordering::Relaxed);
            }
            Err(format!("FSCTL_DUPLICATE_EXTENTS_TO_FILE failed: error {}", err))
        }
    }
}
