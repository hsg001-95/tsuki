use chrono::Utc;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};
use tsuki_actuator::{ActionExecutor, RollbackJournal};
use tsuki_common::intent::ActionState;
use tsuki_common::Config;

#[derive(Parser, Debug)]
#[command(
    name = "tsuki-rollbackd",
    author = "Snoxx Tech",
    version = "0.1.0",
    about = "Tsuki Rollback Watchdog — Guarantees cleanup of expired or orphaned host actions (SAF-04)"
)]
struct Args {
    #[arg(short, long, default_value = "tsuki.toml.example")]
    config: String,

    /// Optional cgroup root path (defaults to /sys/fs/cgroup)
    #[arg(long)]
    cgroup_root: Option<PathBuf>,

    /// Run a single audit and rollback pass then exit
    #[arg(long)]
    oneshot: bool,
}

fn audit_and_revert(journal: &RollbackJournal, executor: &ActionExecutor) -> usize {
    let active = match journal.get_active_actions() {
        Ok(a) => a,
        Err(e) => {
            warn!("Failed to read active journal actions: {}", e);
            return 0;
        }
    };

    let now = Utc::now();
    let mut reverted_count = 0;

    for mut entry in active {
        if entry.expires_at <= now {
            info!(
                "Watchdog detected EXPIRED action [{}] (TTL exceeded). Restoring prior values...",
                entry.action_id
            );
            if let Err(e) = executor.rollback(&entry.intent.target_cgroup, &entry.prior_values) {
                warn!("Watchdog rollback failed for [{}]: {}", entry.action_id, e);
            }

            entry.state = ActionState::Expired;
            entry.completed_at = Some(now);
            entry.rollback_reason = Some("TTL expired under watchdog supervision (SAF-04)".into());

            if let Err(e) = journal.record(&entry) {
                warn!("Watchdog failed to record expiration: {}", e);
            }
            reverted_count += 1;
        }
    }

    reverted_count
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Initializing Tsuki Rollback Watchdog Daemon (v0.1.0)...");
    let cfg = Config::load_from_file(&args.config).unwrap_or_default();

    let journal = RollbackJournal::new(cfg.general.journal_path);
    let cgroup_root = args
        .cgroup_root
        .unwrap_or_else(|| PathBuf::from("/sys/fs/cgroup"));
    let executor = ActionExecutor::new(cgroup_root);

    if args.oneshot {
        let count = audit_and_revert(&journal, &executor);
        println!(
            "Watchdog oneshot audit complete: {} expired actions reverted.",
            count
        );
        return Ok(());
    }

    info!("Watchdog active loop started (polling interval: 5s)...");
    loop {
        let count = audit_and_revert(&journal, &executor);
        if count > 0 {
            info!("Watchdog reverted {} expired actions.", count);
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}
