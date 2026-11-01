pub mod bench;
pub mod clipboard;
pub mod daemon;
pub mod engine;
pub mod menu;
pub mod scanner;
pub mod ui;
pub mod utils;
pub mod verify;

use clap::{Args, Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use engine::strategy::{CopyStrategy, EngineConfig};
use engine::copy_single_file;
use scanner::DirectoryScanner;
use ui::{ProgressReporter, ProgressStats};
use verify::verify_files;

#[derive(Parser)]
#[command(
    name = "hcp",
    author = "HyperCopy Team",
    version = "0.1.0",
    about = "⚡ The world's fastest bare-metal hardware-saturating copy & paste CLI engine written in Rust",
    long_about = "HyperCopy (hcp) extracts every last drop of performance from your hardware using:\n\
                  • ReFS / CoW Instant Zero-I/O Block Cloning (clones gigabytes in <1ms)\n\
                  • Unbuffered Bare-Metal Direct I/O (DMA) bypassing Windows Page Cache\n\
                  • Windows CopyFile2 Hardware Offload (ODX) with local storage optimization\n\
                  • Multi-threaded work-stealing directory scanner with streaming copy pipeline\n\
                  • Seamless Windows Clipboard (CF_HDROP) integration (CLI <-> Explorer Ctrl+V)\n\
                  • Windows Right-Click Context Menu integration for seamless Explorer copy/paste\n\
                  • AVX-512 / AVX2 SIMD BLAKE3 integrity verification (>25 GB/s)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Copy files and directories with maximum hardware saturation
    #[command(alias = "cp")]
    Copy(CopyArgs),

    /// Paste files currently in the Windows system clipboard to destination
    #[command(alias = "p")]
    Paste(PasteArgs),

    /// Clipboard utilities: copy or paste files/text via the Windows system clipboard
    Clip {
        #[command(subcommand)]
        action: ClipAction,
    },

    /// Enable HyperCopy as the system default copy & paste handler (replaces Ctrl+V in Explorer & cp in CLI)
    #[command(alias = "enable-default")]
    EnableDefault,

    /// Disable HyperCopy as the default copy & paste handler
    #[command(alias = "disable-default")]
    DisableDefault,

    /// Run the background interceptor daemon that hooks Explorer Ctrl+V
    Daemon {
        /// Run silently without console banner
        #[arg(long)]
        silent: bool,
    },

    /// Install "Copy with HyperCopy" and "Paste with HyperCopy" into Windows File Explorer context menu
    #[command(alias = "setup-menu")]
    InstallMenu,

    /// Remove HyperCopy from the Windows File Explorer context menu
    #[command(alias = "remove-menu")]
    UninstallMenu,

    /// Run the hardware saturation micro-benchmark to measure SSD & CPU line-rates
    Bench {
        /// Target folder to run benchmark in (defaults to system TEMP)
        target: Option<PathBuf>,
    },

    /// Display detected hardware acceleration capabilities and CPU/storage features
    Info,
}

#[derive(Args, Clone)]
pub struct CopyArgs {
    /// Source file(s) or directory(ies)
    #[arg(required = true)]
    pub sources: Vec<PathBuf>,

    /// Destination file or directory
    #[arg(required = true)]
    pub destination: PathBuf,

    /// Number of concurrent worker threads (defaults to logical CPU cores)
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// Engine strategy: auto, reflink, copyfile2, direct, buffered
    #[arg(short, long, default_value = "auto")]
    pub engine: String,

    /// Buffer size in megabytes for streaming (default: 4 MB)
    #[arg(short = 'b', long = "buffer-size-mb", default_value = "4")]
    pub buffer_size_mb: usize,

    /// Verify file integrity bit-for-bit after copying using AVX SIMD BLAKE3
    #[arg(long)]
    pub verify: bool,

    /// Force unbuffered Direct I/O (bypass OS cache) for all files
    #[arg(long)]
    pub no_buffering: bool,

    /// Suppress live progress dashboard
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Args, Clone)]
pub struct PasteArgs {
    /// Destination directory (defaults to current working directory)
    pub destination: Option<PathBuf>,

    /// Number of concurrent worker threads
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// Engine strategy: auto, reflink, copyfile2, direct, buffered
    #[arg(short, long, default_value = "auto")]
    pub engine: String,

    /// Buffer size in megabytes (default: 4 MB)
    #[arg(short = 'b', long = "buffer-size-mb", default_value = "4")]
    pub buffer_size_mb: usize,

    /// Verify integrity with BLAKE3
    #[arg(long)]
    pub verify: bool,

    /// Force unbuffered Direct I/O
    #[arg(long)]
    pub no_buffering: bool,

    /// Suppress progress display
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum ClipAction {
    /// Copy file(s) or folder(s) to system clipboard (CF_HDROP) for Ctrl+V anywhere
    Copy {
        /// Files or directories to place on the clipboard
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Paste files currently in the clipboard to a destination folder
    Paste {
        /// Destination directory (defaults to current directory)
        destination: Option<PathBuf>,
    },
    /// Copy text string or stdin to system clipboard
    CopyText {
        /// Text to copy (if omitted, reads from stdin)
        text: Option<String>,
    },
    /// Paste text from system clipboard to stdout
    PasteText,
}

pub fn run() {
    utils::enable_ansi_support();
    let cli = Cli::parse();

    match cli.command {
        Commands::Copy(args) => {
            execute_copy(args);
        }
        Commands::Paste(args) => {
            let clipboard_files = match clipboard::get_files_from_clipboard() {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("{} {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            };

            let dest = args.destination.unwrap_or_else(|| PathBuf::from("."));
            println!(
                "{} Found {} item(s) in clipboard. Pasting to {}...",
                "📋".cyan(),
                clipboard_files.len().to_string().bold(),
                dest.display().to_string().bold().yellow()
            );

            let copy_args = CopyArgs {
                sources: clipboard_files,
                destination: dest,
                threads: args.threads,
                engine: args.engine,
                buffer_size_mb: args.buffer_size_mb,
                verify: args.verify,
                no_buffering: args.no_buffering,
                quiet: args.quiet,
            };

            execute_copy(copy_args);
        }
        Commands::Clip { action } => match action {
            ClipAction::Copy { paths } => match clipboard::copy_files_to_clipboard(&paths) {
                Ok(n) => {
                    println!(
                        "{} Placed {} file/folder(s) into the Windows clipboard!",
                        "✅".bold().green(),
                        n.to_string().bold().yellow()
                    );
                    println!(
                        "   {}",
                        "You can now paste with Ctrl+V in File Explorer or run 'hcp paste'!".dimmed()
                    );
                }
                Err(e) => {
                    eprintln!("{} Failed to copy to clipboard: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            },
            ClipAction::Paste { destination } => {
                let dest = destination.unwrap_or_else(|| PathBuf::from("."));
                let clipboard_files = match clipboard::get_files_from_clipboard() {
                    Ok(files) => files,
                    Err(e) => {
                        eprintln!("{} {}", "Error:".bold().red(), e);
                        std::process::exit(1);
                    }
                };

                println!(
                    "{} Pasting {} item(s) from clipboard into {}...",
                    "📋".cyan(),
                    clipboard_files.len(),
                    dest.display()
                );

                let copy_args = CopyArgs {
                    sources: clipboard_files,
                    destination: dest,
                    threads: None,
                    engine: "auto".to_string(),
                    buffer_size_mb: 4,
                    verify: false,
                    no_buffering: false,
                    quiet: false,
                };
                execute_copy(copy_args);
            }
            ClipAction::CopyText { text } => {
                let content = if let Some(t) = text {
                    t
                } else {
                    use std::io::Read;
                    let mut stdin_buf = String::new();
                    let _ = std::io::stdin().read_to_string(&mut stdin_buf);
                    stdin_buf
                };

                match clipboard::copy_text_to_clipboard(&content) {
                    Ok(_) => println!("{} Copied text to clipboard!", "✅".bold().green()),
                    Err(e) => {
                        eprintln!("{} Failed: {}", "Error:".bold().red(), e);
                        std::process::exit(1);
                    }
                }
            }
            ClipAction::PasteText => match clipboard::get_text_from_clipboard() {
                Ok(t) => print!("{}", t),
                Err(e) => {
                    eprintln!("{} Failed: {}", "Error:".bold().red(), e);
                    std::process::exit(1);
                }
            },
        },
        Commands::EnableDefault => {
            if let Err(e) = daemon::enable_default() {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::DisableDefault => {
            if let Err(e) = daemon::disable_default() {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::Daemon { silent } => {
            if let Err(e) = daemon::run_daemon(silent) {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::InstallMenu => {
            if let Err(e) = menu::install_context_menu() {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::UninstallMenu => {
            if let Err(e) = menu::remove_context_menu() {
                eprintln!("{} {}", "Error:".bold().red(), e);
                std::process::exit(1);
            }
        }
        Commands::Bench { target } => {
            bench::run_benchmark(target);
        }
        Commands::Info => {
            print_hardware_info();
        }
    }
}

pub fn execute_copy(args: CopyArgs) {
    let strategy: CopyStrategy = args.engine.parse().unwrap_or_else(|e| {
        eprintln!("{} {}", "Error:".bold().red(), e);
        std::process::exit(1);
    });

    let threads = args.threads.unwrap_or_else(|| (num_cpus::get() * 2).clamp(8, 32));
    let buffer_size = args.buffer_size_mb * 1024 * 1024;
    let no_buf_threshold = if args.no_buffering {
        0u64
    } else {
        8 * 1024 * 1024u64
    };

    let config = Arc::new(EngineConfig {
        strategy,
        buffer_size,
        no_buffering_threshold: no_buf_threshold,
        preallocate: true,
        preserve_attributes: true,
    });

    let stats = ProgressStats::new();

    // Start streaming directory scanner
    let task_rx = DirectoryScanner::scan_and_stream(
        args.sources.clone(),
        args.destination.clone(),
        Arc::clone(&stats),
        10_000,
    );

    // Launch progress reporting UI thread
    let reporter = ProgressReporter::start(Arc::clone(&stats), args.quiet);

    // Build scoped Rayon thread pool for concurrent workers
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to build worker thread pool");

    let verify_enabled = args.verify;

    pool.scope(|s| {
        for _ in 0..threads {
            let rx = task_rx.clone();
            let cfg = Arc::clone(&config);
            let st = Arc::clone(&stats);

            s.spawn(move |_| {
                while let Ok(task) = rx.recv() {
                    let copy_res = copy_single_file(
                        &task.src,
                        &task.dst,
                        task.size,
                        &cfg,
                        &st,
                    );

                    if let Err(e) = copy_res {
                        eprintln!("\n{} {:?} -> {:?}: {}", "Failed to copy".bold().red(), task.src, task.dst, e);
                    } else if verify_enabled {
                        match verify_files(&task.src, &task.dst) {
                            Ok(true) => {}
                            Ok(false) => {
                                eprintln!("\n{} Integrity verification mismatch on {:?}", "CRITICAL:".bold().red(), task.dst);
                                st.failed_files.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(e) => {
                                eprintln!("\n{} Verification failed on {:?}: {}", "Warning:".bold().yellow(), task.dst, e);
                            }
                        }
                    }
                }
            });
        }
    });

    reporter.finish();
}

pub fn print_hardware_info() {
    println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
    println!(" {}", "⚡ HYPERCOPY HARDWARE ACCELERATION PROFILE".bold().bright_yellow());
    println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());
    println!("  {:<28} {}", "Operating System:".bold(), std::env::consts::OS.bold().green());
    println!("  {:<28} {}", "Architecture:".bold(), std::env::consts::ARCH.bold().green());
    println!("  {:<28} {}", "Logical CPU Cores:".bold(), num_cpus::get().to_string().bold().white());
    println!("  {:<28} {}", "Global Memory Allocator:".bold(), "mimalloc (Zero-Lock Thread-Local)".bold().cyan());

    #[cfg(target_arch = "x86_64")]
    {
        let has_avx2 = is_x86_feature_detected!("avx2");
        let has_avx512f = is_x86_feature_detected!("avx512f");
        let has_sse41 = is_x86_feature_detected!("sse4.1");
        println!(
            "  {:<28} AVX2: {}, AVX-512: {}, SSE4.1: {}",
            "SIMD Vector Extensions:".bold(),
            if has_avx2 { "✓ YES".bold().green() } else { "✗ NO".red() },
            if has_avx512f { "✓ YES".bold().green() } else { "✗ NO".dimmed() },
            if has_sse41 { "✓ YES".bold().green() } else { "✗ NO".red() },
        );
    }

    println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());
    println!("  {}:", "Available Storage Acceleration Engines".bold().magenta());
    println!("   • {} - FSCTL_DUPLICATE_EXTENTS_TO_FILE instantaneous CoW", "ReFS Block Cloning:".bold().green());
    println!("   • {} - COPY_FILE_NO_BUFFERING zero-cache DMA transfer", "Kernel Direct Offload:".bold().blue());
    println!("   • {} - Sector-aligned unbuffered Direct DMA I/O", "Bare-Metal Direct I/O:".bold().yellow());
    println!("   • {} - Windows CF_HDROP Explorer & CLI interoperability", "System Clipboard:".bold().cyan());
    println!("   • {} - Multi-threaded AVX2/AVX-512 cryptographic verification", "BLAKE3 SIMD Hash:".bold().bright_magenta());
    println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
}
