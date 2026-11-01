use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use colored::*;
use crate::engine::strategy::{CopyStrategy, EngineConfig};
use crate::engine::{copy_single_file, direct_io, buffered};
use crate::ui::ProgressStats;
use crate::utils::{format_bytes, format_duration, format_speed};

pub fn run_benchmark(target_dir: Option<PathBuf>) {
    let base_dir = target_dir.unwrap_or_else(std::env::temp_dir).join("hypercopy_bench");
    let _ = std::fs::remove_dir_all(&base_dir);
    let _ = std::fs::create_dir_all(&base_dir);

    println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
    println!(" {}", "⚡ HYPERCOPY HARDWARE SATURATION BENCHMARK".bold().bright_yellow());
    println!(" Target workspace: {}", base_dir.display().to_string().cyan());
    println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());

    // Test 1: Small files latency & metadata throughput (500 files, 4KB each)
    println!(" {} Running Small-Files High-Concurrency Test (500 x 4KB)...", "1/2".bold().cyan());
    let small_src_dir = base_dir.join("small_src");
    let small_dst_dir = base_dir.join("small_dst");
    let _ = std::fs::create_dir_all(&small_src_dir);
    let _ = std::fs::create_dir_all(&small_dst_dir);

    let dummy_4k = vec![0xAAu8; 4096];
    for i in 0..500 {
        let fpath = small_src_dir.join(format!("file_{:04}.bin", i));
        let mut f = File::create(&fpath).unwrap();
        f.write_all(&dummy_4k).unwrap();
    }

    let stats_small = ProgressStats::new();
    let config_small = EngineConfig {
        strategy: CopyStrategy::Auto,
        ..Default::default()
    };

    let t0 = Instant::now();
    for i in 0..500 {
        let s = small_src_dir.join(format!("file_{:04}.bin", i));
        let d = small_dst_dir.join(format!("file_{:04}.bin", i));
        let _ = copy_single_file(&s, &d, 4096, &config_small, &stats_small);
    }
    let dur_small = t0.elapsed().as_secs_f64();
    let small_bytes = 500 * 4096u64;
    let small_rate = (small_bytes as f64) / dur_small;
    let files_per_sec = 500.0 / dur_small;

    println!(
        "     Result: {} in {} | {} | {}",
        format_bytes(small_bytes).bold().white(),
        format_duration(dur_small).cyan(),
        format_speed(small_rate).bold().yellow(),
        format!("{:.1} files/sec", files_per_sec).bold().green()
    );

    // Test 2: NVMe Maximum Direct I/O Throughput (150 MB file)
    println!();
    println!(" {} Running Unbuffered Direct DMA Throughput Test (150 MB)...", "2/2".bold().cyan());
    let large_src = base_dir.join("large_src.bin");
    let large_dst = base_dir.join("large_dst.bin");

    let chunk_1mb = vec![0x55u8; 1024 * 1024];
    {
        let mut lf = File::create(&large_src).unwrap();
        for _ in 0..150 {
            lf.write_all(&chunk_1mb).unwrap();
        }
    }

    let large_size = 150 * 1024 * 1024u64;
    let stats_direct = ProgressStats::new();

    let t1 = Instant::now();
    let direct_res = direct_io::direct_io_copy(
        &large_src,
        &large_dst,
        large_size,
        8 * 1024 * 1024,
        &stats_direct,
    );
    let dur_direct = t1.elapsed().as_secs_f64();

    match direct_res {
        Ok(_) => {
            let speed_direct = (large_size as f64) / dur_direct;
            println!(
                "     Unbuffered Direct I/O Speed: {} (Time: {})",
                format_speed(speed_direct).bold().bright_green(),
                format_duration(dur_direct).cyan()
            );
        }
        Err(e) => {
            println!("     Direct I/O skipped ({}). Running standard buffered engine...", e);
            let stats_buf = ProgressStats::new();
            let t_buf = Instant::now();
            let _ = buffered::buffered_copy(
                &large_src,
                &large_dst,
                large_size,
                4 * 1024 * 1024,
                &stats_buf,
            );
            let dur_buf = t_buf.elapsed().as_secs_f64();
            let speed_buf = (large_size as f64) / dur_buf;
            println!(
                "     Buffered Speed: {} (Time: {})",
                format_speed(speed_buf).bold().bright_green(),
                format_duration(dur_buf).cyan()
            );
        }
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&base_dir);

    println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());
    println!(" {}", "✅ BENCHMARK COMPLETE - Hardware limits squeezed successfully!".bold().green());
    println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
}
