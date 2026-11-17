use chrono::Utc;
use std::collections::HashMap;
use tracing::{error, info, warn};
use tsuki_common::config::Config;
use tsuki_common::error::{Result, TsukiError};
use tsuki_common::intent::{ActionState, JournalEntry, RemediationIntent};

use crate::actions::ActionExecutor;
use crate::journal::RollbackJournal;
use crate::validator::IntentValidator;

pub struct RemediationManager {
    config: Config,
    validator: IntentValidator,
    executor: ActionExecutor,
    journal: RollbackJournal,
}

impl RemediationManager {
    pub fn new(
        config: Config,
        validator: IntentValidator,
        executor: ActionExecutor,
        journal: RollbackJournal,
    ) -> Self {
        Self {
            config,
            validator,
            executor,
            journal,
        }
    }

    fn panic_latch_path(&self) -> std::path::PathBuf {
        self.config.general.journal_path.with_extension("panic")
    }

    /// Check if the system is currently under emergency panic lockdown.
    pub fn is_panic_latched(&self) -> bool {
        self.panic_latch_path().exists()
    }

    /// Reset emergency panic latch, restoring actuation capabilities.
    pub fn reset_panic(&self) -> Result<()> {
        let p = self.panic_latch_path();
        if p.exists() {
            std::fs::remove_file(&p)?;
            info!("Emergency panic latch cleared. Actuation restored.");
        }
        Ok(())
    }

    /// Process an incoming intent proposal through the validation and state machine.
    pub fn process_intent(&self, intent: RemediationIntent) -> Result<ActionState> {
        // 0. Persistent Panic Latch Check (SAF-08)
        if self.is_panic_latched() {
            return Err(tsuki_common::error::TsukiError::Validation(
                "Actuation locked: system is in EMERGENCY PANIC state (SAF-08). Reset panic latch before submitting new intents.".into()
            ));
        }

        // 1. Mandatory Deterministic Validation (SAF-02, SAF-03, SAF-05)
        self.validator.validate(&intent)?;

        let now = Utc::now();

        // 2. Cooldown Rate-Limit Check (Rule SAF-07 / Safety Test 8)
        let all_entries = self.journal.read_all_entries()?;
        for e in all_entries.iter().rev() {
            if e.intent.target_cgroup == intent.target_cgroup {
                let elapsed = (now - e.created_at).num_seconds();
                if elapsed >= 0 && (elapsed as u64) < self.config.actuator.cooldown_seconds {
                    return Err(tsuki_common::error::TsukiError::Validation(format!(
                        "Target '{}' is within cooldown window ({}s remaining)",
                        intent.target_cgroup,
                        self.config.actuator.cooldown_seconds - (elapsed as u64)
                    )));
                }
                break;
            }
        }

        let action_id = format!("act-{}", intent.intent_id);
        let expires_at = now + chrono::Duration::seconds(intent.ttl_seconds as i64);

        // 2. Mode Check (SAF-06: Shadow mode default)
        if self.config.general.mode == "shadow" {
            info!(
                "[SHADOW MODE] Intent [{}] validated. Logging action without altering host.",
                intent.intent_id
            );
            let entry = JournalEntry {
                action_id,
                intent,
                state: ActionState::Validated,
                prior_values: HashMap::new(),
                created_at: now,
                expires_at,
                completed_at: Some(now),
                rollback_reason: None,
            };
            self.journal.record(&entry)?;
            return Ok(ActionState::Validated);
        }

        if self.config.general.mode == "approval" {
            info!(
                "[APPROVAL MODE] Intent [{}] awaiting operator confirmation.",
                intent.intent_id
            );
            let entry = JournalEntry {
                action_id,
                intent,
                state: ActionState::AwaitingApproval,
                prior_values: HashMap::new(),
                created_at: now,
                expires_at,
                completed_at: None,
                rollback_reason: None,
            };
            self.journal.record(&entry)?;
            return Ok(ActionState::AwaitingApproval);
        }

        // Autonomous mode
        self.execute_action(&action_id, &intent, expires_at)
    }

    /// Execute an approved action, fsync prior values to journal BEFORE applying to cgroup (SAF-04).
    pub fn execute_action(
        &self,
        action_id: &str,
        intent: &RemediationIntent,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<ActionState> {
        let now = Utc::now();

        // 1. Capture prior values without modifying the host
        let prior_values = self.executor.read_prior_values(intent)?;

        // 2. Journal and fsync BEFORE mutating kernel control files (Rule SAF-04)
        let mut entry = JournalEntry {
            action_id: action_id.to_string(),
            intent: intent.clone(),
            state: ActionState::Applied,
            prior_values: prior_values.clone(),
            created_at: now,
            expires_at,
            completed_at: None,
            rollback_reason: None,
        };
        self.journal.record(&entry)?;

        // 3. Apply actuation to cgroup files
        if let Err(e) = self.executor.apply_with_priors(intent, &prior_values) {
            warn!("Actuation failed for [{}]: {}; rolling back to prior values...", action_id, e);
            if let Err(re) = self.executor.rollback(&intent.target_cgroup, &prior_values) {
                error!("Rollback also failed following actuation failure for [{}]: {}", action_id, re);
            }
            entry.state = ActionState::RolledBack;
            entry.completed_at = Some(Utc::now());
            entry.rollback_reason = Some(format!("Actuation failed: {}", e));
            if let Err(je) = self.journal.record(&entry) {
                error!("Failed to record journal state following actuation failure for [{}]: {}", action_id, je);
            }
            return Err(e);
        }

        info!(
            "Action [{}] applied successfully; state: Observing.",
            action_id
        );
        Ok(ActionState::Observing)
    }

    /// Rollback a specific active action.
    pub fn rollback_action(&self, action_id: &str, reason: &str) -> Result<()> {
        let active = self.journal.get_active_actions()?;
        let target = active.into_iter().find(|e| e.action_id == action_id);

        if let Some(mut entry) = target {
            self.executor
                .rollback(&entry.intent.target_cgroup, &entry.prior_values)?;
            entry.state = ActionState::RolledBack;
            entry.completed_at = Some(Utc::now());
            entry.rollback_reason = Some(reason.to_string());
            self.journal.record(&entry)?;
            info!("Action [{}] rolled back. Reason: {}", action_id, reason);
        }

        Ok(())
    }

    /// Global Panic Switch (SAF-08): Reverts all active actions and permanently latches actuation disabled.
    pub fn panic(&self) -> Result<usize> {
        warn!("🚨 GLOBAL PANIC TRIGGERED: Reverting all active actions immediately (SAF-08)...");

        // 1. Set persistent panic latch on disk (SAF-08)
        let latch_path = self.panic_latch_path();
        if let Some(parent) = latch_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&latch_path, Utc::now().to_rfc3339())?;

        // 2. Revert all currently active actions
        let active = self.journal.get_active_actions()?;
        let count = active.len();
        let mut rollback_errors = Vec::new();

        for mut entry in active {
            if let Err(e) = self
                .executor
                .rollback(&entry.intent.target_cgroup, &entry.prior_values)
            {
                error!("Panic rollback failed for target {}: {}", entry.intent.target_cgroup, e);
                rollback_errors.push(format!("rollback {}: {}", entry.intent.target_cgroup, e));
            }
            entry.state = ActionState::RolledBack;
            entry.completed_at = Some(Utc::now());
            entry.rollback_reason = Some("Emergency global panic triggered".into());
            if let Err(e) = self.journal.record(&entry) {
                error!("Failed to journal panic entry for {}: {}", entry.action_id, e);
                rollback_errors.push(format!("journal record {}: {}", entry.action_id, e));
            }
        }

        if !rollback_errors.is_empty() {
            error!("Panic completed with errors: {:?}", rollback_errors);
            return Err(TsukiError::Rollback(format!(
                "Panic completed with operational errors: {}",
                rollback_errors.join("; ")
            )));
        }

        info!("Panic complete: {} actions reverted. Panic latch active.", count);
        Ok(count)
    }
}
