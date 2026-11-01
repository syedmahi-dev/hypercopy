use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use crate::ui::ProgressStats;

/// High-throughput buffered copy engine with pre-allocation.
/// Ideal for smaller files or filesystems where Direct I/O cannot be bound.
pub fn buffered_copy(
    src: &Path,
    dst: &Path,
    file_size: u64,
    buffer_size: usize,
    stats: &Arc<ProgressStats>,
) -> Result<(), String> {
    let mut reader = File::open(src).map_err(|e| format!("Buffered: Open src error: {}", e))?;
    let mut writer = File::create(dst).map_err(|e| format!("Buffered: Create dst error: {}", e))?;

    // Pre-allocate to prevent file fragmentation
    if file_size > 0 {
        let _ = writer.set_len(file_size);
    }

    let buf_cap = buffer_size.max(64 * 1024);
    let mut buffer = vec![0u8; buf_cap];

    let mut total_read = 0u64;
    while total_read < file_size {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("Buffered: Read error: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        writer
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Buffered: Write error: {}", e))?;

        total_read += bytes_read as u64;
        stats.copied_bytes.fetch_add(bytes_read as u64, Ordering::Relaxed);
    }

    let _ = writer.flush();
    Ok(())
}
