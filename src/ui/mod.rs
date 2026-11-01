use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use colored::*;
use crate::utils::{format_bytes, format_duration, format_speed};

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Default)]
pub struct ProgressStats {
    pub total_files: AtomicU64,
    pub completed_files: AtomicU64,
    pub total_bytes: AtomicU64,
    pub copied_bytes: AtomicU64,
    pub reflink_files: AtomicU64,
    pub direct_io_files: AtomicU64,
    pub copyfile2_files: AtomicU64,
    pub buffered_files: AtomicU64,
    pub failed_files: AtomicU64,
    pub scan_finished: AtomicBool,
}

impl ProgressStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

pub struct ProgressReporter {
    handle: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
    stats: Arc<ProgressStats>,
    start_time: Instant,
    quiet: bool,
}

impl ProgressReporter {
    pub fn start(stats: Arc<ProgressStats>, quiet: bool) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop_signal);
        let stats_clone = Arc::clone(&stats);
        let start_time = Instant::now();

        let handle = if quiet {
            None
        } else {
            Some(thread::spawn(move || {
                let mut last_time = Instant::now();
                let mut last_bytes = 0u64;
                let mut peak_speed = 0.0f64;
                let mut frame_idx = 0usize;

                while !stop_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(40)); // 25 FPS buttery smooth updates

                    let now = Instant::now();
                    let elapsed = (now - last_time).as_secs_f64();
                    if elapsed < 0.035 {
                        continue;
                    }

                    let copied = stats_clone.copied_bytes.load(Ordering::Relaxed);
                    let total = stats_clone.total_bytes.load(Ordering::Relaxed);
                    let completed = stats_clone.completed_files.load(Ordering::Relaxed);
                    let total_f = stats_clone.total_files.load(Ordering::Relaxed);

                    let delta_bytes = copied.saturating_sub(last_bytes);
                    let current_speed = (delta_bytes as f64) / elapsed;
                    if current_speed > peak_speed {
                        peak_speed = current_speed;
                    }

                    last_bytes = copied;
                    last_time = now;

                    let percent = if total > 0 {
                        ((copied as f64 / total as f64) * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };

                    let total_elapsed = (now - start_time).as_secs_f64();
                    let overall_speed = if total_elapsed > 0.0 {
                        (copied as f64) / total_elapsed
                    } else {
                        0.0
                    };

                    let eta_str = if overall_speed > 0.0 && total > copied {
                        let remaining_secs = (total - copied) as f64 / overall_speed;
                        format_duration(remaining_secs)
                    } else {
                        "--".to_string()
                    };

                    // Draw a sleek ASCII/Unicode gauge
                    let width = 24;
                    let filled = ((percent / 100.0) * width as f64).round() as usize;
                    let filled = filled.min(width);
                    let bar: String = "━".repeat(filled) + &"─".repeat(width - filled);

                    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
                    frame_idx = frame_idx.wrapping_add(1);

                    let status_extra = if !stats_clone.scan_finished.load(Ordering::Relaxed) {
                        " (discovering files...)".dimmed()
                    } else if copied >= total && total > 0 && completed < total_f {
                        " (verifying SIMD BLAKE3...)".bright_magenta()
                    } else {
                        "".normal()
                    };

                    // Clean full-line erase followed by updated real-time telemetry
                    eprint!(
                        "\x1b[2K\r{} {} [{}] {:5.1}% | {} / {} | {} | ETA: {} | {}/{} files{}   ",
                        spinner.to_string().bold().bright_cyan(),
                        "HYPERCOPY".bold().cyan(),
                        bar.bold().green(),
                        percent,
                        format_bytes(copied).bold().white(),
                        format_bytes(total).dimmed(),
                        format_speed(current_speed).bold().yellow(),
                        eta_str.cyan(),
                        completed,
                        total_f,
                        status_extra
                    );
                    let _ = std::io::stderr().flush();
                }
            }))
        };

        Self {
            handle,
            stop_signal,
            stats,
            start_time,
            quiet,
        }
    }

    pub fn finish(mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }

        if !self.quiet {
            eprintln!(); // newline to clear the progress bar
            self.print_summary();
        }
    }

    pub fn print_summary(&self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let copied = self.stats.copied_bytes.load(Ordering::Relaxed);
        let completed = self.stats.completed_files.load(Ordering::Relaxed);
        let failed = self.stats.failed_files.load(Ordering::Relaxed);
        let reflink = self.stats.reflink_files.load(Ordering::Relaxed);
        let direct_io = self.stats.direct_io_files.load(Ordering::Relaxed);
        let copyfile2 = self.stats.copyfile2_files.load(Ordering::Relaxed);
        let buffered = self.stats.buffered_files.load(Ordering::Relaxed);

        let avg_speed = if elapsed > 0.0 {
            (copied as f64) / elapsed
        } else {
            0.0
        };

        let files_per_sec = if elapsed > 0.0 {
            (completed as f64) / elapsed
        } else {
            0.0
        };

        println!();
        println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
        println!(
            " {} {}",
            "🚀 HYPERCOPY TRANSFER COMPLETE".bold().bright_green(),
            format!("(squeezed in {})", format_duration(elapsed)).dimmed()
        );
        println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());
        println!("  {:<24} {}", "Total Data Transferred:".bold(), format_bytes(copied).bright_white());
        println!("  {:<24} {}", "Total Files Processed:".bold(), format!("{}", completed).bright_white());
        println!("  {:<24} {}", "Effective Speed:".bold(), format_speed(avg_speed).bold().bright_yellow());
        println!("  {:<24} {}", "Throughput (Files/sec):".bold(), format!("{:.1} files/s", files_per_sec).cyan());
        println!("  {:<24} {}", "Elapsed Time:".bold(), format_duration(elapsed).cyan());

        if failed > 0 {
            println!("  {:<24} {}", "Failed Files:".bold().red(), format!("{}", failed).bold().red());
        }

        println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());
        println!("  {}:", "Hardware Acceleration & Engine Breakdown".bold().magenta());
        if reflink > 0 {
            println!("   • Instant Reflink (CoW zero-I/O): {} files", format!("{}", reflink).bold().green());
        }
        if copyfile2 > 0 {
            println!("   • Kernel Direct Offload:          {} files", format!("{}", copyfile2).bold().blue());
        }
        if direct_io > 0 {
            println!("   • Unbuffered DMA Direct I/O:      {} files", format!("{}", direct_io).bold().yellow());
        }
        if buffered > 0 {
            println!("   • Parallel Pipelined Buffer:      {} files", format!("{}", buffered).dimmed());
        }
        println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
    }
}
