use clap::{Parser, Subcommand};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};
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

    /// Optional cgroup root path (defaults to /sys/fs/cgroup)
    #[arg(long)]
    cgroup_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start actuator daemon listening on Unix domain socket
    Daemon,
    /// Emergency global panic switch (SAF-08): Reverts all active actions immediately
    Panic,
    /// Clear persistent panic latch, restoring actuation capabilities
    ResetPanic,
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

fn build_manager(cfg: &Config, cgroup_root: Option<PathBuf>) -> RemediationManager {
    let root = cgroup_root.unwrap_or_else(|| PathBuf::from("/sys/fs/cgroup"));
    let validator =
        IntentValidator::new(cfg.clone()).with_cgroup_root(root.to_string_lossy().to_string());
    let executor = ActionExecutor::new(root);
    let journal = RollbackJournal::new(cfg.general.journal_path.clone());
    RemediationManager::new(cfg.clone(), validator, executor, journal)
}

fn handle_ipc_request(manager: &RemediationManager, request_json: &str) -> serde_json::Value {
    let req: serde_json::Value = match serde_json::from_str(request_json) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "status": "error",
                "error": format!("Invalid JSON request: {}", e)
            });
        }
    };

    let cmd = req.get("command").and_then(|v| v.as_str()).unwrap_or("");

    match cmd {
        "apply" => {
            let intent_val = match req.get("intent") {
                Some(i) => i.clone(),
                None => {
                    return serde_json::json!({
                        "status": "error",
                        "error": "Missing 'intent' field in apply command"
                    });
                }
            };

            let intent: RemediationIntent = match serde_json::from_value(intent_val) {
                Ok(i) => i,
                Err(e) => {
                    return serde_json::json!({
                        "status": "error",
                        "error": format!("Failed to parse intent: {}", e)
                    });
                }
            };

            match manager.process_intent(intent) {
                Ok(state) => serde_json::json!({
                    "status": "ok",
                    "action_state": format!("{:?}", state),
                    "message": "Intent processed successfully"
                }),
                Err(e) => serde_json::json!({
                    "status": "error",
                    "error": e.to_string()
                }),
            }
        }
        "panic" => match manager.panic() {
            Ok(count) => serde_json::json!({
                "status": "ok",
                "reverted_count": count,
                "message": format!("Global panic executed. {} actions reverted.", count)
            }),
            Err(e) => serde_json::json!({
                "status": "error",
                "error": e.to_string()
            }),
        },
        "reset_panic" => match manager.reset_panic() {
            Ok(_) => serde_json::json!({
                "status": "ok",
                "message": "Panic latch reset successfully. Actuation re-enabled."
            }),
            Err(e) => serde_json::json!({
                "status": "error",
                "error": e.to_string()
            }),
        },
        "status" => {
            let panic_latched = manager.is_panic_latched();
            serde_json::json!({
                "status": "ok",
                "panic_latched": panic_latched,
                "message": if panic_latched { "PANIC_LOCKED" } else { "OPERATIONAL" }
            })
        }
        "rollback" => {
            let action_id = req.get("action_id").and_then(|v| v.as_str()).unwrap_or("");
            let reason = req
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Manual UDS rollback request");

            if action_id.is_empty() {
                return serde_json::json!({
                    "status": "error",
                    "error": "Missing 'action_id' field"
                });
            }

            match manager.rollback_action(action_id, reason) {
                Ok(_) => serde_json::json!({
                    "status": "ok",
                    "message": format!("Action [{}] rolled back", action_id)
                }),
                Err(e) => serde_json::json!({
                    "status": "error",
                    "error": e.to_string()
                }),
            }
        }
        _ => serde_json::json!({
            "status": "error",
            "error": format!("Unknown command: '{}'", cmd)
        }),
    }
}

fn run_uds_daemon(
    manager: Arc<RemediationManager>,
    socket_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = socket_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;
    info!("Tsuki Actuator UDS Daemon listening on {:?}", socket_path);

    for stream in listener.incoming() {
        match stream {
            Ok(mut socket) => {
                let manager_ref = manager.clone();
                let socket_clone = match socket.try_clone() {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Failed to clone stream: {}", e);
                        continue;
                    }
                };

                let mut reader = BufReader::new(socket_clone);
                let mut line = String::new();

                while let Ok(n) = reader.read_line(&mut line) {
                    if n == 0 {
                        break;
                    }
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let resp = handle_ipc_request(&manager_ref, trimmed);
                        let mut resp_str = serde_json::to_string(&resp).unwrap_or_default();
                        resp_str.push('\n');
                        if socket.write_all(resp_str.as_bytes()).is_err() {
                            break;
                        }
                    }
                    line.clear();
                }
            }
            Err(e) => {
                warn!("UDS incoming connection error: {}", e);
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    // Fail closed on invalid configuration (Issue 6)
    let cfg = Config::load_or_default(&cli.config)?;
    let manager = Arc::new(build_manager(&cfg, cli.cgroup_root));

    match cli.command.unwrap_or(Commands::Daemon) {
        Commands::Panic => {
            println!("🚨 Executing global emergency panic switch (SAF-08)...");
            let count = manager.panic()?;
            println!(
                "Panic completed. {} active actions reverted to baseline. Actuation locked.",
                count
            );
        }
        Commands::ResetPanic => {
            println!("Clearing global emergency panic latch...");
            manager.reset_panic()?;
            println!("Panic latch cleared. Actuation re-enabled.");
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
            let latched = manager.is_panic_latched();
            println!("Actuator Status: {}", if latched { "🚨 PANIC_LOCKED" } else { "OPERATIONAL" });
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
            run_uds_daemon(manager, cfg.general.socket_path)?;
        }
    }

    Ok(())
}
