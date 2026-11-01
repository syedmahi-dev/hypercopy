# Contributing to HyperCopy (`hcp`)

Thank you for your interest in making HyperCopy even faster! We welcome contributions, bug reports, and optimizations.

---

## 🛠️ Development Setup

### Prerequisites
- **Rust Toolchain**: `rustup` with Rust 1.85+ (tested on Rust 1.98+).
- **C Compiler**: MSVC (on Windows) or GCC/Clang (on Linux/macOS) for `mimalloc-sys`.

### Building the Project
```bash
# Clone the repository
git clone https://github.com/your-username/hypercopy.git
cd hypercopy

# Build in debug mode
cargo build

# Build fully-optimized release binary with native CPU instructions
cargo build --release
```

---

## 🧪 Testing & Verification

Before opening a pull request, ensure all checks pass:

```bash
# 1. Run unit and integration tests
cargo test

# 2. Check formatting
cargo fmt --all -- --check

# 3. Run Clippy linter
cargo clippy --all-targets -- -D warnings

# 4. Run the internal benchmark suite to ensure no performance regression
cargo run --release -- bench
```

---

## ⚡ Performance Guidelines

HyperCopy is a **performance-critical** system tool. Please adhere to these design principles:

1. **Zero Heap Allocation in Hot Paths**:
   - Never allocate buffers or strings inside the inner chunk read/write loops.
   - Use reusable or stack-allocated memory.
2. **Lock-Free Atomic Counters**:
   - Use `AtomicU64` with `Ordering::Relaxed` for progress tracking. Never acquire a `Mutex` or `RwLock` during byte streaming.
3. **Sector Alignment**:
   - All unbuffered Direct I/O routines must preserve strict physical sector alignment (4096 bytes).
4. **Hardware Offloads First**:
   - Always prefer zero-cost filesystem offloads (Reflink / CoW / Kernel storage offload) before performing manual physical transfers.

---

## 📝 Pull Request Workflow

1. Fork the repository and create your branch from `main`:
   ```bash
   git checkout -b feature/my-optimization
   ```
2. Commit your changes with clear, descriptive commit messages.
3. If introducing an optimization, **include before-and-after benchmark numbers** (run via `hcp bench`).
4. Push to your fork and submit a Pull Request.
