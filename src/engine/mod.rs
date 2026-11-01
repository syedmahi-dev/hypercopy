pub mod buffered;
pub mod copyfile2;
pub mod direct_io;
pub mod parallel_file;
pub mod reflink;
pub mod strategy;

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use crate::ui::ProgressStats;
use crate::utils::path_to_wide;
pub use strategy::{CopyStrategy, EngineConfig};

/// Execute optimized copy for a single file according to the selected engine strategy.
pub fn copy_single_file(
    src: &Path,
    dst: &Path,
    file_size: u64,
    config: &EngineConfig,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    let src_wide = path_to_wide(src);
    let dst_wide = path_to_wide(dst);
    copy_single_file_fast(src, dst, &src_wide, &dst_wide, file_size, config, stats)
}

/// Execute optimized copy with pre-computed wide paths, eliminating UTF-8/UTF-16 conversion overhead.
pub fn copy_single_file_fast(
    src: &Path,
    dst: &Path,
    src_wide: &[u16],
    dst_wide: &[u16],
    file_size: u64,
    config: &EngineConfig,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    match config.strategy {
        CopyStrategy::Parallel => {
            let res = parallel_file::parallel_file_copy(
                src,
                dst,
                src_wide,
                dst_wide,
                file_size,
                config.parallel_streams,
                stats,
            );
            if res.is_ok() {
                stats.direct_io_files.fetch_add(1, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            res
        }
        CopyStrategy::Reflink => {
            let res = reflink::try_reflink_copy(src, dst, file_size);
            if res.is_ok() {
                stats.reflink_files.fetch_add(1, Ordering::Relaxed);
                stats.copied_bytes.fetch_add(file_size, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            res
        }
        CopyStrategy::DirectIo => {
            let res = direct_io::direct_io_copy(src, dst, file_size, config.buffer_size, stats);
            if res.is_ok() {
                stats.direct_io_files.fetch_add(1, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            res
        }
        CopyStrategy::CopyFile2 => {
            let no_buf = file_size >= config.no_buffering_threshold;
            let res = copyfile2::copy_file2_wide(src_wide, dst_wide, file_size, no_buf, stats);
            if res.is_ok() {
                stats.copyfile2_files.fetch_add(1, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            res
        }
        CopyStrategy::Buffered => {
            let res = buffered::buffered_copy(src, dst, file_size, config.buffer_size, stats);
            if res.is_ok() {
                stats.buffered_files.fetch_add(1, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            res
        }
        CopyStrategy::Auto => {
            // Tier 1: Try Reflink (Instant zero-I/O block clone) only for files >= 1 MB if supported by volume
            if file_size >= 1024 * 1024 && reflink::is_reflink_supported() && reflink::try_reflink_copy(src, dst, file_size).is_ok() {
                stats.reflink_files.fetch_add(1, Ordering::Relaxed);
                stats.copied_bytes.fetch_add(file_size, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }

            // Tier 2: Kernel CopyFile2 with pre-computed wide paths & optional unbuffered bypass (>2.2 GB/s)
            let no_buf = file_size >= config.no_buffering_threshold;
            if copyfile2::copy_file2_wide(src_wide, dst_wide, file_size, no_buf, stats).is_ok() {
                stats.copyfile2_files.fetch_add(1, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }

            // Tier 3: Parallel Multi-Part DMA Segment Copy fallback for files >= 64 MB
            if file_size >= 64 * 1024 * 1024 {
                if parallel_file::parallel_file_copy(
                    src,
                    dst,
                    src_wide,
                    dst_wide,
                    file_size,
                    config.parallel_streams,
                    stats,
                ).is_ok() {
                    stats.direct_io_files.fetch_add(1, Ordering::Relaxed);
                    stats.completed_files.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }
            }

            // Tier 4: Direct DMA unbuffered fallback for large files
            if file_size >= config.no_buffering_threshold {
                if direct_io::direct_io_copy(src, dst, file_size, config.buffer_size, stats).is_ok() {
                    stats.direct_io_files.fetch_add(1, Ordering::Relaxed);
                    stats.completed_files.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }
            }

            // Tier 5: Parallel buffered fallback
            let res = buffered::buffered_copy(src, dst, file_size, config.buffer_size, stats);
            if res.is_ok() {
                stats.buffered_files.fetch_add(1, Ordering::Relaxed);
                stats.completed_files.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }

            stats.failed_files.fetch_add(1, Ordering::Relaxed);
            res
        }
    }
}
