use crossbeam_channel::{bounded, Receiver};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use walkdir::WalkDir;
use crate::ui::ProgressStats;

#[derive(Debug, Clone)]
pub struct FileTask {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub size: u64,
}

pub struct DirectoryScanner;

impl DirectoryScanner {
    /// Start streaming scan of source paths to destination.
    /// Emits tasks into a bounded channel so workers can start copying immediately
    /// without waiting for the full directory tree to be indexed.
    pub fn scan_and_stream(
        sources: Vec<PathBuf>,
        destination: PathBuf,
        stats: Arc<ProgressStats>,
        channel_capacity: usize,
    ) -> Receiver<FileTask> {
        let (tx, rx) = bounded::<FileTask>(channel_capacity);

        thread::spawn(move || {
            for src in sources {
                if !src.exists() {
                    eprintln!("Warning: source path does not exist: {:?}", src);
                    continue;
                }

                if src.is_file() {
                    let file_size = src.metadata().map(|m| m.len()).unwrap_or(0);
                    let file_name = src.file_name().unwrap_or_default();
                    let target_dst = if destination.is_dir() || destination.to_string_lossy().ends_with('/') || destination.to_string_lossy().ends_with('\\') {
                        destination.join(file_name)
                    } else {
                        destination.clone()
                    };

                    stats.total_files.fetch_add(1, Ordering::Relaxed);
                    stats.total_bytes.fetch_add(file_size, Ordering::Relaxed);

                    let _ = tx.send(FileTask {
                        src,
                        dst: target_dst,
                        size: file_size,
                    });
                } else if src.is_dir() {
                    let root_name = src.file_name().unwrap_or_default();
                    let target_root = destination.join(root_name);
                    let _ = std::fs::create_dir_all(&target_root);

                    for entry in WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let rel = match path.strip_prefix(&src) {
                            Ok(r) => r,
                            Err(_) => continue,
                        };

                        let target_path = target_root.join(rel);

                        if entry.file_type().is_dir() {
                            let _ = std::fs::create_dir_all(&target_path);
                        } else if entry.file_type().is_file() {
                            let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            stats.total_files.fetch_add(1, Ordering::Relaxed);
                            stats.total_bytes.fetch_add(file_size, Ordering::Relaxed);

                            let _ = tx.send(FileTask {
                                src: path.to_path_buf(),
                                dst: target_path,
                                size: file_size,
                            });
                        }
                    }
                }
            }

            stats.scan_finished.store(true, Ordering::Relaxed);
            // tx drops here, closing the channel for workers
        });

        rx
    }
}
