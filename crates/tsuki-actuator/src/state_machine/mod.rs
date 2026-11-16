use chrono::Utc;
use std::collections::HashMap;
use tracing::{info, warn};
use tsuki_common::config::Config;
use tsuki_common::error::Result;
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

    /// Process an incoming intent proposal through the validation and state machine.
    pub fn process_intent(&self, intent: RemediationIntent) -> Result<ActionState> {
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
            let _ = self.executor.rollback(&intent.target_cgroup, &prior_values);
            entry.state = ActionState::RolledBack;
            entry.completed_at = Some(Utc::now());
            entry.rollback_reason = Some(format!("Actuation failed: {}", e));
            let _ = self.journal.record(&entry);
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

    /// Global Panic Switch (SAF-08): Reverts all active actions and disables actuation.
    pub fn panic(&self) -> Result<usize> {
        warn!("🚨 GLOBAL PANIC TRIGGERED: Reverting all active actions immediately (SAF-08)...");
        let active = self.journal.get_active_actions()?;
        let count = active.len();

        for mut entry in active {
            let _ = self
                .executor
                .rollback(&entry.intent.target_cgroup, &entry.prior_values);
            entry.state = ActionState::RolledBack;
            entry.completed_at = Some(Utc::now());
            entry.rollback_reason = Some("Emergency global panic triggered".into());
            let _ = self.journal.record(&entry);
        }

        info!("Panic complete: {} actions reverted.", count);
        Ok(count)
    }
}
