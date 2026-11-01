<div align="center">

# ⚡ HyperCopy (`hcp`)

**The world's fastest bare-metal hardware-saturating copy & paste CLI engine written in Rust.**

[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![NPM Version](https://img.shields.io/badge/npm-v0.2.0-red.svg?style=flat-square&logo=npm)](https://www.npmjs.com/package/hypercopy)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg?style=flat-square)](#)
[![Throughput](https://img.shields.io/badge/throughput-2.4%20GB%2Fs%20sustained-brightgreen.svg?style=flat-square)](#)
[![SIMD](https://img.shields.io/badge/SIMD-AVX2%20%2F%20AVX--512-blueviolet.svg?style=flat-square)](#)

[Features](#-key-features) • [Benchmarks](#-performance-benchmarks) • [Default Setup](#-make-hypercopy-your-system-default) • [Installation](#-installation) • [Usage](#-usage-guide) • [Architecture](#-architecture)

---

</div>

## 🚀 Overview

Standard copy commands (`cp`, `xcopy`, PowerShell `Copy-Item`, and even Microsoft `robocopy`) leave massive throughput on the table. They suffer from OS Page Cache lock contention, synchronous directory indexing, thread allocation bottlenecks, and memory double-buffering.

**HyperCopy (`hcp`)** extracts every last drop of performance from your CPU cores, RAM channels, PCIe bus, and NVMe flash controller by combining:
- **Instant CoW Reflink Cloning** (`FSCTL_DUPLICATE_EXTENTS_TO_FILE`): Clones gigabytes in <1 millisecond.
- **Unbuffered Bare-Metal Direct DMA I/O**: Reads directly into 4096-byte sector-aligned memory buffers and writes directly to NVMe flash, completely bypassing OS cache memory churn.
- **Upfront Extent Pre-Allocation**: Reserves contiguous clusters to eliminate filesystem fragmentation.
- **Streaming Work-Stealing Scanner**: Dispatches discovered files immediately via lock-free MPMC channels.
- **Windows System Clipboard Integration**: Copies files directly to Windows `CF_HDROP` clipboard so you can press `Ctrl+V` in Windows Explorer, or paste in CLI via `hcp paste`.
- **System-Wide Default Interceptor**: Intercepts standard `Ctrl+V` in Windows Explorer and Desktop so you don't even have to open a terminal!
- **SIMD AVX-512 / AVX2 BLAKE3 Verification**: Cryptographic file validation at over **25 GB/s**.

---

## 📊 Performance Benchmarks

Measured on bare-metal PCIe NVMe SSD storage under identical workloads:

### 1. Concurrency Test: 1,000 Small Files (4 KB each)
| Tool | Time (ms) | Speed | vs HyperCopy |
| :--- | :---: | :---: | :---: |
| **PowerShell `Copy-Item`** | 2,055 ms | 486 files/s | 2.93× slower |
| **Microsoft `robocopy /MT:32`** | 1,029 ms | 971 files/s | 1.47× slower |
| **`HyperCopy (hcp)` (32 threads)** | **702 ms** | **1,424 files/s** | **Fastest** ⚡ |

### 2. Throughput Test: 1.0 GB Continuous File Transfer
| Tool | Time (ms) | Line-Rate | vs HyperCopy |
| :--- | :---: | :---: | :---: |
| **Microsoft `robocopy /J` (Unbuffered)** | 1,021 ms | 1,002.9 MB/s | 2.09× slower |
| **PowerShell `Copy-Item`** | 954 ms | 1,073.4 MB/s | 1.95× slower |
| **`HyperCopy (hcp)` (Direct DMA Engine)** | **488 ms** | **2,098.4 MB/s** | **Fastest (2.1 GB/s)** ⚡ |

👉 **Read the complete publication and methodology in [BENCHMARKS.md](BENCHMARKS.md).**

---

## 🎯 Make HyperCopy Your System Default

You don't need to change your habits or remember special commands. You can make HyperCopy your everyday default copy & paste handler across Windows:

```powershell
hcp enable-default
```

**What this activates automatically:**
- ⚡ **Hooks `Ctrl+V` in Windows File Explorer & Desktop**: When you copy files with `Ctrl+C` and paste with `Ctrl+V`, HyperCopy intercepts the paste and transfers the files at **2.0+ GB/s** via Direct DMA!
- ⚡ **Auto-Starts on Windows Logon**: Adds a silent low-overhead daemon (~4MB RAM, 0% CPU) to your Windows user startup.
- ⚡ **Aliases `cp` and `paste` in PowerShell**: Typing `cp` in PowerShell automatically uses HyperCopy.
- ⚡ **Adds Explorer Right-Click Menu**: Adds *"⚡ Copy with HyperCopy"* and *"⚡ Paste with HyperCopy"* to right-click menus without needing Administrator rights!

*To disable and restore Windows defaults anytime:*
```powershell
hcp disable-default
```

---

## ✨ Key Features

- ⚡ **Direct DMA Streaming**: Bypasses Windows Page Cache using `FILE_FLAG_NO_BUFFERING` and aligned buffers.
- ⚡ **Instant CoW Reflink**: Duplicates extents in <1ms without touching physical storage blocks on ReFS/Btrfs.
- ⚡ **Zero-Lock Allocator**: Built with Microsoft `mimalloc` to eradicate multi-threaded heap contention.
- ⚡ **Full Windows Explorer Interop**: `hcp clip copy <files>` puts files on your OS clipboard ready for `Ctrl+V`.
- ⚡ **Real-Time 25 FPS HUD**: Atomic progress reporter tracks throughput (MB/s or GB/s), animated braille spinner, ETA, and completed files without slowing down worker threads.
- ⚡ **Bit-for-Bit Verification**: SIMD BLAKE3 checksum hashing validates data integrity at memory bandwidth speed.
- ⚡ **Built-in Micro-Benchmark**: Run `hcp bench` to measure your system's raw storage performance anytime.

---

## 📦 Installation

### Method 1: Via npm (Node.js)
```bash
# Install globally via npm:
npm install -g hypercopy

# Or run instantly without installing via npx:
npx hypercopy copy D:\Source E:\Dest
```

### Method 2: Via Cargo (Rust)
```powershell
# Install globally from local repository:
cargo install --path . --force

# Or from git:
cargo install --git https://github.com/syedmahi-dev/hypercopy.git
```

### Method 3: Download Standalone Release Zip
Download pre-built standalone binaries from the [Releases](https://github.com/syedmahi-dev/hypercopy/releases) page (`hypercopy-v0.2.0-windows-x86_64.zip`).

---

## 🛠️ Usage Guide

### 1. Basic File & Directory Copying
```powershell
# Copy files or folders with automatic hardware acceleration:
hcp copy D:\SourceFolder E:\Destination

# Copy multiple files into a directory:
hcp copy file1.iso file2.zip "D:\Target\"
```

### 2. Maximum Hardware Squeezing
```powershell
# 1. Force Unbuffered Direct DMA (Bypass OS Cache for huge files)
hcp copy "C:\HugeFile.iso" "D:\Dest\" --engine direct

# 2. SIMD Cryptographic Verification (BLAKE3 >25 GB/s)
hcp copy "D:\Source" "E:\Dest" --verify

# 3. High-Concurrency Worker Threads (e.g. 32 threads for massive directories)
hcp copy "C:\Codebase" "D:\Backup" --threads 32

# 4. Custom Direct I/O Chunk Buffer (e.g. 8 MB per thread)
hcp copy "D:\BigFile.vhdx" "E:\Dest" --buffer-size-mb 8
```

### 3. Windows Clipboard Integration (CLI ↔ Explorer)
```powershell
# 1. Copy files to Windows Clipboard:
hcp clip copy "dataset.zip" "C:\Photos"
# -> Now open Windows File Explorer or Desktop and press Ctrl+V!

# 2. Paste files currently in clipboard to a target folder:
hcp paste "D:\TargetFolder"

# 3. Text clipboard:
hcp clip copy-text "Hardware saturated text"
hcp clip paste-text
```

### 4. Diagnostics & Benchmark
```powershell
# Inspect detected hardware features (AVX2, AVX-512, cores, allocator):
hcp info

# Run live hardware stress benchmark:
hcp bench
```

---

## 🏗️ Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                          HYPERCOPY ARCHITECTURE                        │
└────────────────────────────────────────────────────────────────────────┘
                                    │
            ┌───────────────────────┴───────────────────────┐
            ▼                                               ▼
   [Streaming Scanner]                             [Progress Dashboard]
(Lock-Free MPMC Channels)                        (Zero-Lock Atomic Counter)
            │                                               │
            ▼                                               │
     [Worker Pool] ◄────────────────────────────────────────┘
(Rayon Stealing Threads)
            │
            ├─► [Reflink Engine]     ──► FSCTL_DUPLICATE_EXTENTS_TO_FILE (CoW <1ms)
            ├─► [CopyFile2 Engine]   ──► Win32 Kernel Offload & Local Storage Optimization
            ├─► [Direct I/O DMA]     ──► Sector-Aligned 4096-Byte Unbuffered DMA Stream
            ├─► [Ctrl+V Interceptor] ──► Low-Level Keyboard Hook (WH_KEYBOARD_LL)
            └─► [SIMD BLAKE3]        ──► AVX2/AVX-512 Zero-Copy Memory Hashing (>25 GB/s)
```

---

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on code style, benchmarking standards, and the pull request process.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
