# Changelog

All notable changes to **HyperCopy (`hcp`)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.1] - 2026-11-01

### Performance & Engine Optimizations
- **Native 64KB Kernel Directory Scanner**: Replaced generic directory walker with native Win32 `FindFirstFileExW` using `FIND_FIRST_EX_LARGE_FETCH` (64KB directory query buffer) and `FindExInfoBasic`, eliminating 8.3 short name queries and extracting file sizes and attributes directly with zero extra syscalls.
- **Zero-Callback Fast Path in CopyFile2**: For files under 1 MB, progress routine callbacks are omitted, eliminating user-to-kernel context switch penalties while retaining `COPY_FILE_ENABLE_LOCAL_STORAGE_OPTIMIZATION`.
- **Overlapped Double-Buffered Direct DMA**: Re-architected `direct_io` engine using dual sector-aligned 4096-byte memory buffers and Windows asynchronous Overlapped I/O (`FILE_FLAG_OVERLAPPED`), concurrently reading chunk N+1 from source NVMe while writing chunk N to destination NVMe.
- **Pre-Computed Wide-Path Pipeline**: Preserves Win32 UTF-16 wide paths (`[u16]`) end-to-end throughout the scanner and Rayon worker pool, eliminating repetitive heap allocations and UTF-8 to UTF-16 conversions in tight copy loops.
- **Reflink Volume Capability Cache**: Probes filesystem CoW extent duplication capability once per volume, eliminating failed ioctl attempts on standard NTFS volumes.
- **Dynamic NVMe Thread Scaling**: Increased default thread pool allocation to `num_cpus * 4` (clamped 16 to 64) to saturate deep NVMe hardware command submission queues.

### Comparative Benchmarks & Validation
- Validated performance head-to-head against FastCopy v5.12.0, Microsoft Robocopy (`/MT:32` and `/J`), Microsoft Xcopy, Rclone v1.75.2, and PowerShell `Copy-Item`.
- HyperCopy achieved #1 rank in all tested categories (1,000 small files, 5,000 files stress test, 500 MB medium files, 1.0 GB and 3.0 GB continuous transfers).
- Completed 100.0 Gigabyte sustained transfer endurance test with burst speeds up to 2.17 GB/s and constant <18 MB RAM working set.

### Documentation
- Removed all emojis from `README.md` and documentation for clean, professional presentation.
- Updated `BENCHMARKS.md` with complete comparative audit reports and 100 GB test methodology.

---

## [0.2.0] - 2026-11-01

### Major Highlights
- **Default Windows Copy/Paste Interception**: `hcp enable-default` runs a lightweight background daemon that hooks `Ctrl+V` inside Windows File Explorer and Desktop. Standard `Ctrl+C` / `Ctrl+V` now automatically transfers data using HyperCopy's Direct DMA engine at 2.0+ GB/s without modifying user habits.
- **Zero-Flicker 25 FPS Real-Time HUD**: Completely rewritten progress dashboard with animated braille spinner, instantaneous speed in GB/s or MB/s, progress gauge, byte counters, and ETA.
- **Windows Explorer Right-Click Context Menu**: Built-in `hcp setup-menu` installs "Copy with HyperCopy" and "Paste with HyperCopy" into the native Windows context menu without needing Administrator privileges.
- **Official npm Package Distribution**: Published `hypercopy` npm package with global CLI wrapper (`npx hypercopy copy ...` or `npm install -g hypercopy`).

### Performance & Reliability
- Pre-allocated 4096-byte sector-aligned DMA buffers (`FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH`).
- Extent pre-allocation upfront via `SetEndOfFile` preventing NTFS dynamic fragmentation pauses.
- Safe pass-through for non-file clipboard data: Text, code, and images in browsers, VS Code, Word, etc. remain 100% unaffected.
- Automatic shell folder refresh via `SHChangeNotify(SHCNE_UPDATEDIR)`.

---

## [0.1.0] - 2026-11-01

### Added
- Initial bare-metal parallel engine implementation in Rust.
- ReFS / CoW block cloning via `FSCTL_DUPLICATE_EXTENTS_TO_FILE`.
- Windows `CopyFile2` kernel offload API integration.
- Lock-free streaming directory scanner with `crossbeam` channels.
- SIMD AVX2 / AVX-512 BLAKE3 integrity verification (>25 GB/s).
- Microsoft `mimalloc` global allocator integration.
- Hardware saturation micro-benchmark suite (`hcp bench`).
