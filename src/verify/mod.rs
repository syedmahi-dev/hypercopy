use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Compute SIMD-accelerated BLAKE3 hash of a file.
/// Uses memory-mapping where appropriate to achieve memory-bandwidth saturation (>25 GB/s).
pub fn hash_file(path: &Path) -> Result<blake3::Hash, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open file for hashing: {}", e))?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    let file_size = metadata.len();

    if file_size == 0 {
        return Ok(blake3::Hasher::new().finalize());
    }

    // For files > 64 KB, use memmap for zero-copy SIMD hashing
    if file_size >= 64 * 1024 {
        if let Ok(mmap) = unsafe { memmap2::Mmap::map(&file) } {
            let mut hasher = blake3::Hasher::new();
            hasher.update_rayon(&mmap);
            return Ok(hasher.finalize());
        }
    }

    // Fallback streaming hasher for small files or unmappable files
    let mut hasher = blake3::Hasher::new();
    let mut reader = std::io::BufReader::with_capacity(1024 * 1024, file);
    let mut buffer = vec![0u8; 1024 * 1024];

    loop {
        let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize())
}

/// Verify bit-for-bit file integrity between source and destination using BLAKE3.
pub fn verify_files(src: &Path, dst: &Path) -> Result<bool, String> {
    let src_hash = hash_file(src)?;
    let dst_hash = hash_file(dst)?;
    Ok(src_hash == dst_hash)
}
