# Changelog

All notable changes to **HyperCopy (`hcp`)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.0] - 2026-11-01

### 🚀 Major Highlights
- **Default Windows Copy/Paste Interception**: `hcp enable-default` runs a lightweight background daemon that hooks `Ctrl+V` inside Windows File Explorer and Desktop. Standard `Ctrl+C` / `Ctrl+V` now automatically transfers data using HyperCopy's Direct DMA engine at 2.0+ GB/s without modifying user habits!
- **Zero-Flicker 25 FPS Real-Time HUD**: Completely rewritten progress dashboard with animated braille spinner (`⠋ ⠙ ⠹ ...`), instantaneous speed in GB/s or MB/s, progress gauge, byte counters, and ETA.
- **Windows Explorer Right-Click Context Menu**: Built-in `hcp setup-menu` installs "⚡ Copy with HyperCopy" and "⚡ Paste with HyperCopy" into the native Windows context menu without needing Administrator privileges.
- **Official npm Package Distribution**: Published `hypercopy` npm package with global CLI wrapper (`npx hypercopy copy ...` or `npm install -g hypercopy`).

### ⚡ Performance & Reliability
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
