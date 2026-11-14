use chrono::Utc;
use clap::Parser;
use std::time::Duration;
use tracing::info;
use tsuki_common::intent::ActionState;
use tsuki_common::Config;

// Re-use ActionExecutor and RollbackJournal concepts from tsuki-actuator
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

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

    /// Run a single audit and rollback pass then exit
    #[arg(long)]
    oneshot: bool,
}

fn rollback_target(
    cgroup_root: &Path,
    target_cgroup: &str,
    prior_values: &HashMap<String, String>,
) {
    let trimmed = target_cgroup.trim_start_matches('/');
    let target_dir = cgroup_root.join(trimmed);
    if !target_dir.exists() {
        return;
    }

    info!("Watchdog rolling back expired target: {:?}", target_dir);
    for (filename, prior_val) in prior_values {
        let p = target_dir.join(filename);
        if p.exists() {
            let _ = fs::write(p, prior_val);
        }
    }
}

fn audit_and_revert(journal_path: &Path, cgroup_root: &Path) -> usize {
    if !journal_path.exists() {
        return 0;
    }

    let file = match File::open(journal_path) {
        Ok(f) => f,
        Err(_) => return 0,
    };

    let reader = BufReader::new(file);
    let mut latest_entries = HashMap::new();

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<tsuki_common::intent::JournalEntry>(trimmed) {
            latest_entries.insert(entry.action_id.clone(), entry);
        }
    }

    let now = Utc::now();
    let mut reverted_count = 0;

    for (action_id, mut entry) in latest_entries {
        if (entry.state == ActionState::Applied || entry.state == ActionState::Observing)
            && entry.expires_at <= now
        {
            info!(
                "Watchdog detected EXPIRED action [{}] (TTL exceeded). Restoring prior values...",
                action_id
            );
            rollback_target(
                cgroup_root,
                &entry.intent.target_cgroup,
                &entry.prior_values,
            );

            entry.state = ActionState::Expired;
            entry.completed_at = Some(now);
            entry.rollback_reason = Some("TTL expired under watchdog supervision (SAF-04)".into());

            // Append completion to journal
            if let Ok(mut j_file) = fs::OpenOptions::new().append(true).open(journal_path) {
                if let Ok(mut line) = serde_json::to_string(&entry) {
                    line.push('\n');
                    let _ = j_file.write_all(line.as_bytes());
                    let _ = j_file.sync_all();
                }
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

    let journal_path = cfg.general.journal_path;
    let cgroup_root = PathBuf::from("/sys/fs/cgroup");

    info!("Monitoring journal: {:?}", journal_path);

    if args.oneshot {
        let count = audit_and_revert(&journal_path, &cgroup_root);
        println!(
            "Watchdog oneshot audit complete: {} expired actions reverted.",
            count
        );
        return Ok(());
    }

    info!("Watchdog active loop started (polling interval: 5s)...");
    loop {
        let count = audit_and_revert(&journal_path, &cgroup_root);
        if count > 0 {
            info!("Watchdog reverted {} expired actions.", count);
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}
