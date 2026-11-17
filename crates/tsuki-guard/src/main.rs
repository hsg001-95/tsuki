pub mod collector;
pub mod detector;
pub mod features;
pub mod sampler;
pub mod snapshot;

use clap::Parser;
use collector::Collector;
use nix::sys::mman::{mlockall, MlockAllFlags};
use sampler::Sampler;
use snapshot::SnapshotRingBuffer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use tsuki_common::Config;

#[derive(Parser, Debug)]
#[command(
    name = "tsuki-guard",
    author = "Snoxx Tech",
    version = "0.1.0",
    about = "Tsuki Guard Plane Daemon — Local-first Linux fault detector & early warning engine"
)]
struct Args {
    #[arg(short, long, default_value = "tsuki.toml.example")]
    config: String,

    /// Collect a single telemetry sample, print summary, and exit
    #[arg(long)]
    oneshot: bool,

    /// Run overhead benchmark for N seconds and output performance statistics
    #[arg(long)]
    bench_duration: Option<u64>,
}

fn try_mlockall() {
    // Attempt mlockall per PERF-01 to avoid page faults during saturation
    match mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
        Ok(_) => info!("Memory locked successfully via mlockall (PERF-01)"),
        Err(e) => {
            warn!(
                "mlockall failed (insufficient privileges or RLIMIT_MEMLOCK): {}. Continuing without lock.",
                e
            );
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Initializing Tsuki Guard Plane (v0.1.0)...");
    let cfg = Config::load_or_default(&args.config)?;

    try_mlockall();

    let collector = Collector::default();
    let ring_buffer = SnapshotRingBuffer::new(cfg.general.snapshot_dir.clone(), 20);

    let running = Arc::new(AtomicBool::new(true));
    install_signal_handlers(running.clone())?;

    let detector = detector::ModelDetector::new(std::path::PathBuf::from("models/detector.onnx"));
    let mut sampler = Sampler::new(cfg.guard.clone(), collector, ring_buffer, running.clone())
        .with_detector(detector);

    if args.oneshot {
        info!("Running oneshot telemetry collection...");
        let (feat, det) = sampler.step()?;
        println!("\n=== Tsuki Guard Oneshot Telemetry ===");
        println!(
            "Memory Pressure : some={:.2}% full={:.2}% (delta={:.2}%/s)",
            feat.psi_mem_some_avg10, feat.psi_mem_full_avg10, feat.psi_mem_some_delta
        );
        println!(
            "CPU Pressure    : some={:.2}% (delta={:.2}%/s)",
            feat.psi_cpu_some_avg10, feat.psi_cpu_some_delta
        );
        println!(
            "IO Pressure     : some={:.2}% full={:.2}%",
            feat.psi_io_some_avg10, feat.psi_io_full_avg10
        );
        println!("Refault Rate    : {}/s", feat.refault_rate);
        println!("Direct Scan Rate: {}/s", feat.pgscan_direct_rate);
        if let Some(cg) = feat.primary_cgroup {
            println!(
                "Top Cgroup      : {} (RAM: {:.1} MB)",
                cg,
                feat.primary_cgroup_memory_bytes as f64 / (1024.0 * 1024.0)
            );
        }
        println!(
            "Detection State : [{}] score={:.2} alert={}",
            det.fault_type, det.anomaly_score, det.is_alert
        );
        println!("=====================================\n");
        return Ok(());
    }

    if let Some(secs) = args.bench_duration {
        info!("Running overhead benchmark for {} seconds...", secs);
        let start_time = Instant::now();
        let mut total_samples = 0u64;
        let mut max_latency_us = 0u128;
        let mut sum_latency_us = 0u128;

        while start_time.elapsed().as_secs() < secs && running.load(Ordering::Relaxed) {
            let step_start = Instant::now();
            let _ = sampler.step()?;
            let step_dur = step_start.elapsed().as_micros();

            total_samples += 1;
            sum_latency_us += step_dur;
            if step_dur > max_latency_us {
                max_latency_us = step_dur;
            }

            std::thread::sleep(std::time::Duration::from_millis(100)); // 10 Hz stress benchmark
        }

        let avg_latency_us = if total_samples > 0 {
            sum_latency_us / (total_samples as u128)
        } else {
            0
        };

        println!("\n=== Tsuki Guard Overhead Benchmark Results ===");
        println!("Total Samples Collected : {}", total_samples);
        println!(
            "Average Step Latency    : {:.2} ms (budget: <= {} ms)",
            avg_latency_us as f64 / 1000.0,
            cfg.guard.inference_p99_budget_ms
        );
        println!(
            "Max Step Latency        : {:.2} ms",
            max_latency_us as f64 / 1000.0
        );
        println!(
            "Overhead Budget Status  : {}",
            if avg_latency_us < (cfg.guard.inference_p99_budget_ms as u128 * 1000) {
                "PASSED ✅"
            } else {
                "FAILED ❌"
            }
        );
        println!("==============================================\n");
        return Ok(());
    }

    info!(
        "Starting active daemon mode in '{}' mode...",
        cfg.general.mode
    );
    sampler.run()?;

    Ok(())
}

fn install_signal_handlers(running: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    RUNNING_FLAG.store(true, Ordering::SeqCst);
    unsafe {
        let _ = nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGINT,
            nix::sys::signal::SigHandler::Handler(handle_sigint),
        );
        let _ = nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGTERM,
            nix::sys::signal::SigHandler::Handler(handle_sigint),
        );
    }

    // Spawn a thread to monitor the global atomic flag and signal the local Arc
    std::thread::spawn(move || {
        while RUNNING_FLAG.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        running.store(false, Ordering::SeqCst);
    });

    Ok(())
}

static RUNNING_FLAG: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_sigint(_: i32) {
    RUNNING_FLAG.store(false, Ordering::SeqCst);
}
