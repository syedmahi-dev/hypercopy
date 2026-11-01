# 📖 HyperCopy (`hcp`) Usability & Configuration Guide

HyperCopy is an ultra-fast, hardware-saturating copy & paste tool written in Rust. It utilizes **Unbuffered Direct DMA I/O**, **ReFS Copy-On-Write**, **Windows CopyFile2 Hardware Offload**, and **SIMD BLAKE3 Verification**.

---

## 🚀 1. Installation

HyperCopy is already compiled and installed globally in your `~/.cargo/bin` directory.

To reinstall or update at any time:
```powershell
# From the project directory:
cd C:\Users\Mahi\.gemini\antigravity\scratch\hypercopy
cargo install --path . --force
```

Verify the installation from any terminal:
```powershell
hcp --version
# Output: hcp 0.1.0
```

---

## ⚡ 2. Setting HyperCopy as Your Default Copy Tool

### Option A: Make `cp` and `paste` Default in PowerShell

Add these lines to your PowerShell profile so `cp` automatically uses `hcp`:

Run in PowerShell:
```powershell
Add-Content $PROFILE @"

# HyperCopy: Default Ultra-Fast Copy & Paste
Remove-Item Alias:cp -ErrorAction SilentlyContinue
function cp { & hcp copy @args }
function paste { & hcp paste @args }
"@
```

Now in any new terminal:
- `cp "source" "destination"` automatically runs `hcp copy`!
- `paste "destination"` automatically pastes from clipboard!

---

### Option B: Add to Windows Explorer Right-Click Menu

To right-click any file or folder in File Explorer and copy/paste with HyperCopy:

Run this in an Administrator PowerShell prompt:
```powershell
# Add "Copy with HyperCopy" for files
New-Item -Path "Registry::HKEY_CLASSES_ROOT\*\shell\HyperCopy" -Value "Copy with HyperCopy" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\*\shell\HyperCopy" -Name "Icon" -Value "shell32.dll,134"
New-Item -Path "Registry::HKEY_CLASSES_ROOT\*\shell\HyperCopy\command" -Value "`"C:\Users\Mahi\.cargo\bin\hcp.exe`" clip copy `"%1`"" -Force | Out-Null

# Add "Copy with HyperCopy" for directories
New-Item -Path "Registry::HKEY_CLASSES_ROOT\Directory\shell\HyperCopy" -Value "Copy with HyperCopy" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\Directory\shell\HyperCopy" -Name "Icon" -Value "shell32.dll,134"
New-Item -Path "Registry::HKEY_CLASSES_ROOT\Directory\shell\HyperCopy\command" -Value "`"C:\Users\Mahi\.cargo\bin\hcp.exe`" clip copy `"%1`"" -Force | Out-Null

# Add "Paste with HyperCopy" for folder background
New-Item -Path "Registry::HKEY_CLASSES_ROOT\Directory\Background\shell\HyperCopyPaste" -Value "Paste with HyperCopy" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\Directory\Background\shell\HyperCopyPaste" -Name "Icon" -Value "shell32.dll,260"
New-Item -Path "Registry::HKEY_CLASSES_ROOT\Directory\Background\shell\HyperCopyPaste\command" -Value "`"C:\Users\Mahi\.cargo\bin\hcp.exe`" paste `"%V`"" -Force | Out-Null
```

---

## 🛠️ 3. Command Reference & Usability

### Command 1: `hcp copy` (or `hcp cp`)
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

### Command 2: `hcp paste` (or `hcp p`)
Pastes files or folders currently stored in the Windows system clipboard (`CF_HDROP`).

```powershell
# Paste clipboard contents into the specified folder:
hcp paste D:\DestinationFolder

# Paste into the current directory:
hcp paste
```

---

### Command 3: `hcp clip`
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

### Command 4: `hcp bench`
Runs the live hardware saturation benchmark to test IOPS and throughput:

```powershell
hcp bench
```

---

### Command 5: `hcp info`
Displays detected hardware acceleration profile (CPU cores, SIMD instructions, allocator):

```powershell
hcp info
```
