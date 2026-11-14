use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};
use tsuki_actuator::{ActionExecutor, IntentValidator, RemediationManager, RollbackJournal};
use tsuki_common::intent::RemediationIntent;
use tsuki_common::Config;

#[derive(Parser, Debug)]
#[command(
    name = "tsuki-actuator",
    author = "Snoxx Tech",
    version = "0.1.0",
    about = "Tsuki Actuator — Deterministic validation and reversible actuation engine (Plane 2)"
)]
struct Cli {
    #[arg(short, long, default_value = "tsuki.toml.example")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start actuator daemon listening on Unix domain socket
    Daemon,
    /// Emergency global panic switch (SAF-08): Reverts all active actions immediately
    Panic,
    /// Apply a validated remediation intent from a JSON string or file
    Apply {
        #[arg(short, long)]
        file: Option<PathBuf>,
        #[arg(short, long)]
        json: Option<String>,
    },
    /// Revert a specific applied action by ID
    Rollback {
        #[arg(short, long)]
        action_id: String,
        #[arg(short, long, default_value = "Manual rollback request")]
        reason: String,
    },
    /// List all currently active remediation actions in the journal
    Status,
}

fn build_manager(cfg: &Config) -> RemediationManager {
    let validator = IntentValidator::new(cfg.clone());
    let executor = ActionExecutor::default();
    let journal = RollbackJournal::new(cfg.general.journal_path.clone());
    RemediationManager::new(cfg.clone(), validator, executor, journal)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let cfg = Config::load_from_file(&cli.config).unwrap_or_default();
    let manager = build_manager(&cfg);

    match cli.command.unwrap_or(Commands::Daemon) {
        Commands::Panic => {
            println!("🚨 Executing global emergency panic switch (SAF-08)...");
            let count = manager.panic()?;
            println!(
                "Panic completed. {} active actions reverted to baseline.",
                count
            );
        }
        Commands::Apply { file, json } => {
            let content = match (file, json) {
                (Some(p), _) => fs::read_to_string(p)?,
                (_, Some(s)) => s,
                (None, None) => {
                    error!("Must provide either --file or --json");
                    std::process::exit(1);
                }
            };

            let intent: RemediationIntent = serde_json::from_str(&content)?;
            println!("Processing remediation intent [{}]...", intent.intent_id);
            let state = manager.process_intent(intent)?;
            println!("Resulting Action State: {:?}", state);
        }
        Commands::Rollback { action_id, reason } => {
            println!("Rolling back action [{}]...", action_id);
            manager.rollback_action(&action_id, &reason)?;
            println!("Rollback completed.");
        }
        Commands::Status => {
            let journal = RollbackJournal::new(cfg.general.journal_path);
            let active = journal.get_active_actions()?;
            println!("Active Reversible Actions ({}):", active.len());
            for a in active {
                println!(
                    " - [{}] target='{}' expires_at='{}'",
                    a.action_id, a.intent.target_cgroup, a.expires_at
                );
            }
        }
        Commands::Daemon => {
            info!("Starting Tsuki Actuator Daemon (v0.1.0)");
            info!("Mode: {}", cfg.general.mode);
            info!("Rollback Journal: {:?}", cfg.general.journal_path);
            info!("Protected cgroups: {:?}", cfg.protected.cgroups);
            info!("Actuator ready. Standing by for intents.");
        }
    }

    Ok(())
}
