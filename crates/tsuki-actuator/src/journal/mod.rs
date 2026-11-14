use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use tracing::{debug, warn};
use tsuki_common::error::Result;
use tsuki_common::intent::{ActionState, JournalEntry};

/// Append-only, fsynced rollback journal.
/// Implements Rule SAF-04 & SAF-08: Every applied action must be journaled before execution
/// to guarantee safe recovery even after system restart or crash.
#[derive(Debug, Clone)]
pub struct RollbackJournal {
    journal_path: PathBuf,
}

impl Default for RollbackJournal {
    fn default() -> Self {
        Self {
            journal_path: PathBuf::from("/var/lib/tsuki/rollback.journal"),
        }
    }
}

impl RollbackJournal {
    pub fn new(journal_path: PathBuf) -> Self {
        Self { journal_path }
    }

    pub fn init(&self) -> io::Result<()> {
        if let Some(parent) = self.journal_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !self.journal_path.exists() {
            let _ = File::create(&self.journal_path)?;
        }
        Ok(())
    }

    /// Append an entry to the journal and fsync immediately (SAF-04).
    pub fn record(&self, entry: &JournalEntry) -> Result<()> {
        self.init()?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)?;

        let mut json_line = serde_json::to_string(entry)?;
        json_line.push('\n');

        file.write_all(json_line.as_bytes())?;
        file.sync_all()?; // Mandatory fsync before kernel actuation

        debug!(
            "Journaled action [{}] state: {:?}",
            entry.action_id, entry.state
        );
        Ok(())
    }

    /// Read all journal entries from the append-only log.
    pub fn read_all_entries(&self) -> Result<Vec<JournalEntry>> {
        if !self.journal_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.journal_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (idx, line) in reader.lines().enumerate() {
            let l = match line {
                Ok(s) => s,
                Err(e) => {
                    warn!("IO error reading journal line {}: {}", idx, e);
                    continue;
                }
            };
            let trimmed = l.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<JournalEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(e) => warn!("Corrupt journal entry at line {}: {}", idx, e),
            }
        }

        Ok(entries)
    }

    /// Retrieve active actions that are currently Applied or Observing.
    pub fn get_active_actions(&self) -> Result<Vec<JournalEntry>> {
        let all = self.read_all_entries()?;
        let mut active_map = std::collections::HashMap::new();

        // The latest entry for an action_id determines its current state
        for e in all {
            active_map.insert(e.action_id.clone(), e);
        }

        let active = active_map
            .into_values()
            .filter(|e| e.state == ActionState::Applied || e.state == ActionState::Observing)
            .collect();

        Ok(active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use tsuki_common::intent::{ActionTemplate, RemediationIntent};

    #[test]
    fn test_journal_record_and_replay() {
        let temp_dir = std::env::temp_dir().join("tsuki_test_journal");
        let _ = fs::remove_dir_all(&temp_dir);
        let journal_path = temp_dir.join("rollback.journal");

        let journal = RollbackJournal::new(journal_path);
        let now = Utc::now();

        let intent = RemediationIntent {
            intent_id: "int-01".into(),
            template: ActionTemplate::ThrottleMemory,
            target_cgroup: "/app.slice".into(),
            params: HashMap::new(),
            rationale: "test".into(),
            evidence_ids: vec![],
            ttl_seconds: 300,
        };

        let entry = JournalEntry {
            action_id: "act-01".into(),
            intent,
            state: ActionState::Applied,
            prior_values: HashMap::new(),
            created_at: now,
            expires_at: now + chrono::Duration::seconds(300),
            completed_at: None,
            rollback_reason: None,
        };

        journal.record(&entry).unwrap();

        let active = journal.get_active_actions().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].action_id, "act-01");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
