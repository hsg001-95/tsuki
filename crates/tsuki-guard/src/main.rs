use clap::Parser;
use tracing::info;
use tsuki_common::Config;

#[derive(Parser, Debug)]
#[command(name = "tsuki-guard", author = "Snoxx Tech", version = "0.1.0", about = "Tsuki Guard Plane Daemon")]
struct Args {
    #[arg(short, long, default_value = "/etc/tsuki/tsuki.toml")]
    config: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Starting Tsuki Guard Plane Daemon (v0.1.0)");
    let cfg = Config::load_from_file(&args.config).unwrap_or_else(|_| {
        info!("Config file not found or invalid; falling back to safe default configuration");
        Config::default()
    });

    info!("Running in mode: {}", cfg.general.mode);
    info!("Sampling rate: {} Hz (adaptive={})", cfg.guard.sampling_rate_hz, cfg.guard.adaptive_sampling);

    Ok(())
}
