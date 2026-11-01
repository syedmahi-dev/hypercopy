use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashSet;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    FILE_ATTRIBUTE_DIRECTORY, FIND_FIRST_EX_LARGE_FETCH, WIN32_FIND_DATAW,
};
use crate::ui::ProgressStats;
use crate::utils::path_to_wide;

#[derive(Debug, Clone)]
pub struct FileTask {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub src_wide: Vec<u16>,
    pub dst_wide: Vec<u16>,
    pub size: u64,
}

pub struct DirectoryScanner;

impl DirectoryScanner {
    /// Start streaming scan of source paths to destination.
    /// Uses native Windows FindFirstFileExW with FIND_FIRST_EX_LARGE_FETCH (64KB buffer)
    /// and FindExInfoBasic (avoids 8.3 short name overhead).
    /// Emits tasks into a bounded channel so workers start copying immediately
    /// without waiting for the full directory tree to be indexed.
    pub fn scan_and_stream(
        sources: Vec<PathBuf>,
        destination: PathBuf,
        stats: Arc<ProgressStats>,
        channel_capacity: usize,
    ) -> Receiver<FileTask> {
        let (tx, rx) = bounded::<FileTask>(channel_capacity);

        thread::spawn(move || {
            let mut created_dirs = HashSet::new();

            for src in sources {
                if !src.exists() {
                    eprintln!("Warning: source path does not exist: {:?}", src);
                    continue;
                }

                if src.is_file() {
                    let file_size = src.metadata().map(|m| m.len()).unwrap_or(0);
                    let file_name = src.file_name().unwrap_or_default();
                    let target_dst = if destination.is_dir()
                        || destination.to_string_lossy().ends_with('/')
                        || destination.to_string_lossy().ends_with('\\')
                    {
                        destination.join(file_name)
                    } else {
                        destination.clone()
                    };

                    // Ensure destination directory exists
                    if let Some(parent) = target_dst.parent() {
                        if !created_dirs.contains(parent) {
                            let _ = std::fs::create_dir_all(parent);
                            created_dirs.insert(parent.to_path_buf());
                        }
                    }

                    stats.total_files.fetch_add(1, Ordering::Relaxed);
                    stats.total_bytes.fetch_add(file_size, Ordering::Relaxed);

                    let src_wide = path_to_wide(&src);
                    let dst_wide = path_to_wide(&target_dst);

                    let _ = tx.send(FileTask {
                        src,
                        dst: target_dst,
                        src_wide,
                        dst_wide,
                        size: file_size,
                    });
                } else if src.is_dir() {
                    let root_name = src.file_name().unwrap_or_default();
                    let target_root = destination.join(root_name);

                    // Ensure target root exists
                    let _ = std::fs::create_dir_all(&target_root);
                    created_dirs.insert(target_root.clone());

                    Self::scan_directory_fast(&src, &target_root, &tx, &stats, &mut created_dirs);
                }
            }

            stats.scan_finished.store(true, Ordering::Relaxed);
            // tx drops here, closing the channel for workers
        });

        rx
    }

    /// High-performance Win32 directory walker using large kernel fetch buffers
    /// and pre-computing UTF-16 wide paths to avoid string encoding conversions during copying.
    fn scan_directory_fast(
        src_dir: &PathBuf,
        dst_dir: &PathBuf,
        tx: &Sender<FileTask>,
        stats: &Arc<ProgressStats>,
        created_dirs: &mut HashSet<PathBuf>,
    ) {
        let mut dir_queue: Vec<(PathBuf, PathBuf)> = vec![(src_dir.clone(), dst_dir.clone())];

        while let Some((curr_src, curr_dst)) = dir_queue.pop() {
            let search_pattern = curr_src.join("*");
            let pattern_wide = path_to_wide(&search_pattern);

            unsafe {
                let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();

                let handle = FindFirstFileExW(
                    pattern_wide.as_ptr(),
                    FindExInfoBasic,
                    &mut find_data as *mut _ as *mut _,
                    FindExSearchNameMatch,
                    std::ptr::null(),
                    FIND_FIRST_EX_LARGE_FETCH,
                );

                if handle == INVALID_HANDLE_VALUE {
                    continue;
                }

                loop {
                    // Check for "." and ".."
                    let is_dot = find_data.cFileName[0] == ('.' as u16)
                        && (find_data.cFileName[1] == 0
                            || (find_data.cFileName[1] == ('.' as u16)
                                && find_data.cFileName[2] == 0));

                    if !is_dot {
                        let len = find_data
                            .cFileName
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(find_data.cFileName.len());
                        let file_name = OsString::from_wide(&find_data.cFileName[..len]);

                        let child_src = curr_src.join(&file_name);
                        let child_dst = curr_dst.join(&file_name);

                        let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;

                        if is_dir {
                            if !created_dirs.contains(&child_dst) {
                                let _ = std::fs::create_dir_all(&child_dst);
                                created_dirs.insert(child_dst.clone());
                            }
                            dir_queue.push((child_src, child_dst));
                        } else {
                            let file_size = ((find_data.nFileSizeHigh as u64) << 32)
                                | (find_data.nFileSizeLow as u64);

                            let src_wide = path_to_wide(&child_src);
                            let dst_wide = path_to_wide(&child_dst);

                            stats.total_files.fetch_add(1, Ordering::Relaxed);
                            stats.total_bytes.fetch_add(file_size, Ordering::Relaxed);

                            if tx
                                .send(FileTask {
                                    src: child_src,
                                    dst: child_dst,
                                    src_wide,
                                    dst_wide,
                                    size: file_size,
                                })
                                .is_err()
                            {
                                FindClose(handle);
                                return;
                            }
                        }
                    }

                    if FindNextFileW(handle, &mut find_data) == 0 {
                        break;
                    }
                }

                FindClose(handle);
            }
        }
    }
}
