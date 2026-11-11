use clap::Parser;
use tracing::info;
use tsuki_common::Config;

#[derive(Parser, Debug)]
#[command(
    name = "tsuki-rollbackd",
    author = "Snoxx Tech",
    version = "0.1.0",
    about = "Tsuki Rollback Watchdog"
)]
struct Args {
    #[arg(short, long, default_value = "/etc/tsuki/tsuki.toml")]
    config: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Starting Tsuki Rollback Watchdog (v0.1.0)");
    let cfg = Config::load_from_file(&args.config).unwrap_or_default();

    info!(
        "Monitoring rollback journal at: {:?}",
        cfg.general.journal_path
    );

    Ok(())
}
