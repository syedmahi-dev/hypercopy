use std::ffi::c_void;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use windows_sys::Win32::Storage::FileSystem::{
    CopyFile2, COPYFILE2_CALLBACK_CHUNK_FINISHED, COPYFILE2_EXTENDED_PARAMETERS,
    COPYFILE2_MESSAGE, COPYFILE2_PROGRESS_CONTINUE,
};
use crate::ui::ProgressStats;
use crate::utils::path_to_wide;

pub const COPY_FILE_NO_BUFFERING: u32 = 0x00001000;
pub const COPY_FILE_ENABLE_LOCAL_STORAGE_OPTIMIZATION: u32 = 0x00008000;

pub type HRESULT = i32;

struct CopyProgressContext {
    last_transferred: AtomicU64,
    stats: *const ProgressStats,
}

unsafe extern "system" fn copyfile2_progress_routine(
    p_message: *const COPYFILE2_MESSAGE,
    pv_callback_context: *const c_void,
) -> i32 {
    if p_message.is_null() || pv_callback_context.is_null() {
        return COPYFILE2_PROGRESS_CONTINUE;
    }
    unsafe {
        let msg = &*p_message;
        if msg.Type == COPYFILE2_CALLBACK_CHUNK_FINISHED {
            let ctx = &*(pv_callback_context as *const CopyProgressContext);
            let current_total = msg.Info.ChunkFinished.uliTotalBytesTransferred;
            let prev = ctx.last_transferred.swap(current_total, Ordering::Relaxed);
            let delta = current_total.saturating_sub(prev);
            if delta > 0 {
                (*ctx.stats).copied_bytes.fetch_add(delta, Ordering::Relaxed);
            }
        }
    }
    COPYFILE2_PROGRESS_CONTINUE
}

/// Execute high-speed copy using Windows CopyFile2 kernel offload API.
/// If `no_buffering` is true, OS file system cache is bypassed, eliminating memory churn.
pub fn copy_file2(
    src: &Path,
    dst: &Path,
    file_size: u64,
    no_buffering: bool,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    let src_wide = path_to_wide(src);
    let dst_wide = path_to_wide(dst);

    let mut copy_flags = COPY_FILE_ENABLE_LOCAL_STORAGE_OPTIMIZATION;
    if no_buffering {
        copy_flags |= COPY_FILE_NO_BUFFERING;
    }

    let ctx = CopyProgressContext {
        last_transferred: AtomicU64::new(0),
        stats: Arc::as_ptr(stats),
    };

    let params = COPYFILE2_EXTENDED_PARAMETERS {
        dwSize: std::mem::size_of::<COPYFILE2_EXTENDED_PARAMETERS>() as u32,
        dwCopyFlags: copy_flags,
        pfCancel: std::ptr::null_mut(),
        pProgressRoutine: Some(copyfile2_progress_routine),
        pvCallbackContext: &ctx as *const _ as *mut c_void,
    };

    let hr: HRESULT = unsafe { CopyFile2(src_wide.as_ptr(), dst_wide.as_ptr(), &params) };

    if hr >= 0 {
        // Ensure any remaining bytes are accounted for
        let finished = ctx.last_transferred.load(Ordering::Relaxed);
        if file_size > finished {
            stats
                .copied_bytes
                .fetch_add(file_size - finished, Ordering::Relaxed);
        }
        Ok(())
    } else {
        Err(format!("CopyFile2 failed with HRESULT 0x{:08X}", hr as u32))
    }
}
