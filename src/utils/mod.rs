pub mod aligned_buf;

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

/// Enable ANSI virtual terminal processing on Windows consoles
pub fn enable_ansi_support() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        };

        unsafe {
            for handle_id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                let handle = GetStdHandle(handle_id);
                if handle != std::ptr::null_mut() && handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                    let mut mode: u32 = 0;
                    if GetConsoleMode(handle, &mut mode) != 0 {
                        let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                    }
                }
            }
        }
    }
}

/// Convert a path to a null-terminated UTF-16 wide string for Windows API calls.
pub fn path_to_wide(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

/// Convert string slice to wide null-terminated vector
pub fn str_to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

/// Format bytes into human-readable string (e.g. 1.45 GB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Format throughput in bytes per second (e.g. 7.42 GB/s)
pub fn format_speed(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:.2} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.2} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.2} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

/// Format duration in seconds to mm:ss or fractional seconds
pub fn format_duration(secs: f64) -> String {
    if secs < 1.0 {
        format!("{:.1}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.2}s", secs)
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let rem = secs - (mins as f64 * 60.0);
        format!("{}m {:04.1}s", mins, rem)
    }
}
