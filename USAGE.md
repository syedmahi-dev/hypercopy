# 📖 HyperCopy (`hcp`) Usability & Configuration Guide

HyperCopy is an ultra-fast, hardware-saturating copy & paste tool written in Rust. It utilizes **Unbuffered Direct DMA I/O**, **ReFS Copy-On-Write**, **Windows CopyFile2 Hardware Offload**, and **SIMD BLAKE3 Verification**.

---

## 🎯 1. Setting HyperCopy as Your System Default (Recommended)

You don't need to learn new commands or open special menus. You can make HyperCopy your everyday default copy & paste handler across Windows:

```powershell
hcp enable-default
```

### What this activates automatically:
1. **Windows File Explorer & Desktop**: Intercepts standard `Ctrl+V` whenever files are on the clipboard, transferring them via Direct DMA at **2.0+ GB/s**!
2. **Windows Startup**: Runs the lightweight background interceptor daemon (~4 MB RAM, 0% CPU) silently on boot.
3. **PowerShell Profile**: Configures `cp` and `paste` aliases in `$PROFILE` so terminal commands run HyperCopy.
4. **Context Menu**: Adds *"⚡ Copy with HyperCopy"* and *"⚡ Paste with HyperCopy"* to Windows File Explorer right-click menus without needing Administrator privileges.

*To disable and restore Windows defaults at any time:*
```powershell
hcp disable-default
```

---

## 🚀 2. Installation Options

### Option A: Via npm
```bash
npm install -g hypercopy
# Or run on demand without installing:
npx hypercopy copy D:\Source E:\Dest
```

### Option B: Via Cargo (Rust)
```powershell
cargo install --path . --force
# Or from GitHub:
cargo install --git https://github.com/syedmahi-dev/hypercopy.git
```

Verify the installation from any terminal:
```powershell
hcp --version
# Output: hcp 0.2.0
```

---

## 🛠️ 3. Command Reference & Usability

### Command: `hcp copy` (or `hcp cp`)
Copies files or directories using the fastest hardware engine.

```powershell
# Basic copy (auto-selects best engine: CoW -> Direct DMA -> Kernel Offload)
hcp copy <source...> <destination>

# Examples:
hcp copy D:\LargeFile.iso E:\Backup\
hcp copy C:\SourceDirectory D:\TargetDirectory
hcp copy file1.zip file2.tar D:\Destination\
```

#### Flags:
- `--engine <auto|direct|copyfile2|reflink|buffered>`:
  - `auto` *(default)*: Tiers Reflink (CoW) -> CopyFile2 -> Direct DMA -> Buffered.
  - `direct`: Force unbuffered direct DMA I/O (completely bypasses OS cache; ideal for NVMe SSDs).
  - `copyfile2`: Windows kernel offloaded transfer.
  - `reflink`: Instant block cloning (<1ms on ReFS).
  - `buffered`: Parallel buffered transfer.
- `--verify`: Computes AVX2/AVX-512 BLAKE3 hashes of source and destination to guarantee 100% bit-exact integrity.
- `-t, --threads <N>`: Worker thread pool count (default: auto-tuned to CPU cores * 2).
- `-b, --buffer-size-mb <N>`: Direct I/O memory buffer per thread in MB (default: 4 MB).
- `-q, --quiet`: Disable interactive live gauge.

---

### Command: `hcp paste` (or `hcp p`)
Pastes files or folders currently stored in the Windows system clipboard (`CF_HDROP`).

```powershell
# Paste clipboard contents into the specified folder:
hcp paste D:\DestinationFolder

# Paste into the current directory:
hcp paste
```

---

### Command: `hcp clip`
Clipboard interoperability with Windows File Explorer and terminal text.

```powershell
# Put files/folders into Windows clipboard (ready for Ctrl+V in Explorer):
hcp clip copy "file.iso" "folder\"

# Paste files from clipboard into a folder:
hcp clip paste D:\Target

# Copy text to clipboard:
hcp clip copy-text "My text"
cat file.txt | hcp clip copy-text

# Output text from clipboard:
hcp clip paste-text
```

---

### Command: `hcp bench`
Runs the live hardware saturation benchmark to test IOPS and throughput on your machine:

```powershell
hcp bench
```

---

### Command: `hcp info`
Displays detected hardware acceleration profile (CPU cores, SIMD instructions, allocator):

```powershell
hcp info
```
