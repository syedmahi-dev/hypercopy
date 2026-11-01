# Changelog

All notable changes to **HyperCopy (`hcp`)** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-11-01

### Added
- **Instant CoW Reflink Engine**: Support for `FSCTL_DUPLICATE_EXTENTS_TO_FILE` allowing instantaneous zero-I/O cloning on ReFS and modern volumes.
- **Bare-Metal Direct DMA Unbuffered I/O**: Direct I/O streaming using `FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH` with 4096-byte sector-aligned memory buffers, bypassing OS Page Cache thrashing.
- **Windows Kernel Storage Offload (`CopyFile2`)**: Integration with Microsoft ODX (Offloaded Data Transfer) and `COPY_FILE_ENABLE_LOCAL_STORAGE_OPTIMIZATION`.
- **Pre-Allocation Engine**: Automatic contiguous cluster reservation via `SetEndOfFile` upfront to eradicate filesystem fragmentation.
- **Streaming Directory Scanner**: Asynchronous work-stealing directory traversal using bounded lock-free `crossbeam` channels, streaming discovered files immediately to workers.
- **Windows System Clipboard Integration (`CF_HDROP`)**: True copy-paste interop allowing files to be copied in CLI and pasted in File Explorer (or vice-versa).
- **SIMD BLAKE3 Verification**: AVX2 and AVX-512 cryptographic file verification operating at >25 GB/s.
- **Zero-Lock Allocator**: Integrated Microsoft `mimalloc` as global memory allocator to eliminate thread lock contention.
- **Zero-Overhead HUD**: Atomic real-time progress reporter updating at 15 FPS without slowing down active worker threads.
- **Hardware Diagnostics & Benchmarks**: Built-in `hcp bench` and `hcp info` subcommands for system inspection.
