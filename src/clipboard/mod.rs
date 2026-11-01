use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
};
use windows_sys::Win32::UI::Shell::DragQueryFileW;

pub const CF_UNICODETEXT: u32 = 13;
pub const CF_HDROP: u32 = 15;

#[repr(C)]
struct DROPFILES {
    p_files: u32,
    pt: POINT,
    f_nc: i32,
    f_wide: i32,
}

/// Place files into the Windows system clipboard using standard CF_HDROP format.
/// Once copied, they can be pasted anywhere: inside Windows File Explorer (Ctrl+V),
/// in third-party apps, or via `hypercopy paste`.
pub fn copy_files_to_clipboard(paths: &[PathBuf]) -> Result<usize, String> {
    if paths.is_empty() {
        return Err("No paths provided to copy to clipboard".to_string());
    }

    // Build the UTF-16 double-null-terminated buffer
    let mut file_list_wide: Vec<u16> = Vec::new();
    let mut valid_count = 0;

    for path in paths {
        let abs_path = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => path.clone(),
        };

        // Strip Windows extended-length prefix (\\?\) for standard shell compatibility
        let path_str = abs_path.to_string_lossy();
        let clean_str = if path_str.starts_with(r"\\?\") {
            &path_str[4..]
        } else {
            &path_str
        };

        let wide: Vec<u16> = std::ffi::OsStr::new(clean_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        file_list_wide.extend(wide);
        valid_count += 1;
    }

    // Append final terminating null character
    file_list_wide.push(0);

    let dropfiles_size = std::mem::size_of::<DROPFILES>();
    let total_bytes = dropfiles_size + (file_list_wide.len() * std::mem::size_of::<u16>());

    unsafe {
        let h_global = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_bytes);
        if h_global.is_null() {
            return Err("Failed to allocate global clipboard memory".to_string());
        }

        let p_mem = GlobalLock(h_global) as *mut u8;
        if p_mem.is_null() {
            return Err("Failed to lock global clipboard memory".to_string());
        }

        let dropfiles = DROPFILES {
            p_files: dropfiles_size as u32,
            pt: POINT { x: 0, y: 0 },
            f_nc: 0,
            f_wide: 1, // UTF-16 wide strings
        };

        // Write DROPFILES header
        std::ptr::copy_nonoverlapping(
            &dropfiles as *const _ as *const u8,
            p_mem,
            dropfiles_size,
        );

        // Write file path strings immediately following the header
        let p_strings = p_mem.add(dropfiles_size) as *mut u16;
        std::ptr::copy_nonoverlapping(
            file_list_wide.as_ptr(),
            p_strings,
            file_list_wide.len(),
        );

        GlobalUnlock(h_global);

        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open Windows clipboard".to_string());
        }

        EmptyClipboard();

        let set_res = SetClipboardData(CF_HDROP, h_global);
        CloseClipboard();

        if set_res.is_null() {
            return Err("Failed to set clipboard CF_HDROP data".to_string());
        }

        Ok(valid_count)
    }
}

/// Retrieve list of files and folders currently in the Windows system clipboard (CF_HDROP).
pub fn get_files_from_clipboard() -> Result<Vec<PathBuf>, String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open Windows clipboard".to_string());
        }

        let h_data = GetClipboardData(CF_HDROP);
        if h_data.is_null() {
            CloseClipboard();
            return Err("No files or folders found in the clipboard (CF_HDROP is empty)".to_string());
        }

        let file_count = DragQueryFileW(h_data as *mut _, 0xFFFFFFFF, std::ptr::null_mut(), 0);
        let mut paths = Vec::with_capacity(file_count as usize);

        for i in 0..file_count {
            let len = DragQueryFileW(h_data as *mut _, i, std::ptr::null_mut(), 0) as usize;
            if len > 0 {
                let mut buffer: Vec<u16> = vec![0; len + 1];
                DragQueryFileW(h_data as *mut _, i, buffer.as_mut_ptr(), (len + 1) as u32);
                buffer.pop(); // Remove null terminator
                let os_str = OsString::from_wide(&buffer);
                paths.push(PathBuf::from(os_str));
            }
        }

        CloseClipboard();
        Ok(paths)
    }
}

/// Copy text string to Windows clipboard (CF_UNICODETEXT).
pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let wide: Vec<u16> = std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let total_bytes = wide.len() * std::mem::size_of::<u16>();

    unsafe {
        let h_global = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total_bytes);
        if h_global.is_null() {
            return Err("Failed to allocate global memory".to_string());
        }

        let p_mem = GlobalLock(h_global) as *mut u16;
        if p_mem.is_null() {
            return Err("Failed to lock global memory".to_string());
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr(), p_mem, wide.len());
        GlobalUnlock(h_global);

        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open clipboard".to_string());
        }

        EmptyClipboard();
        let res = SetClipboardData(CF_UNICODETEXT, h_global);
        CloseClipboard();

        if res.is_null() {
            Err("Failed to set clipboard text".to_string())
        } else {
            Ok(())
        }
    }
}

/// Get text from Windows clipboard (CF_UNICODETEXT).
pub fn get_text_from_clipboard() -> Result<String, String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open clipboard".to_string());
        }

        let h_data = GetClipboardData(CF_UNICODETEXT);
        if h_data.is_null() {
            CloseClipboard();
            return Err("No text found in clipboard".to_string());
        }

        let p_mem = GlobalLock(h_data) as *const u16;
        if p_mem.is_null() {
            CloseClipboard();
            return Err("Failed to lock clipboard data".to_string());
        }

        let mut len = 0;
        while *p_mem.add(len) != 0 {
            len += 1;
        }

        let slice = std::slice::from_raw_parts(p_mem, len);
        let text = OsString::from_wide(slice).to_string_lossy().into_owned();

        GlobalUnlock(h_data);
        CloseClipboard();

        Ok(text)
    }
}
