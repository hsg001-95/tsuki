use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use tsuki_common::error::{Result, TsukiError};
use tsuki_common::intent::{ActionTemplate, RemediationIntent};

/// Action Executor executing allow-listed, reversible cgroup changes (SAF-04).
#[derive(Debug, Clone)]
pub struct ActionExecutor {
    cgroup_root: PathBuf,
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self {
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
        }
    }
}

impl ActionExecutor {
    pub fn new(cgroup_root: PathBuf) -> Self {
        Self { cgroup_root }
    }

    fn resolve_path(&self, target_cgroup: &str) -> PathBuf {
        let trimmed = target_cgroup.trim_start_matches('/');
        self.cgroup_root.join(trimmed)
    }

    /// Read the prior value of a cgroup control file before modifying it.
    fn read_prior_value(&self, path: &Path) -> io::Result<String> {
        let mut file = File::open(path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        Ok(buf.trim().to_string())
    }

    /// Read the prior values of relevant cgroup control files before modification.
    pub fn read_prior_values(&self, intent: &RemediationIntent) -> Result<HashMap<String, String>> {
        let target_dir = self.resolve_path(&intent.target_cgroup);
        if !target_dir.exists() {
            return Err(TsukiError::Validation(format!(
                "Target cgroup directory '{:?}' does not exist",
                target_dir
            )));
        }

        let mut prior_values = HashMap::new();
        match intent.template {
            ActionTemplate::ThrottleMemory => {
                let mem_high_file = target_dir.join("memory.high");
                let prior = self
                    .read_prior_value(&mem_high_file)
                    .unwrap_or_else(|_| "max".into());
                prior_values.insert("memory.high".into(), prior);
            }
            ActionTemplate::ReweightCpu => {
                let cpu_weight_file = target_dir.join("cpu.weight");
                let prior = self
                    .read_prior_value(&cpu_weight_file)
                    .unwrap_or_else(|_| "100".into());
                prior_values.insert("cpu.weight".into(), prior);
            }
            ActionTemplate::FreezeCgroup => {
                let freeze_file = target_dir.join("cgroup.freeze");
                let prior = self
                    .read_prior_value(&freeze_file)
                    .unwrap_or_else(|_| "0".into());
                prior_values.insert("cgroup.freeze".into(), prior);
            }
        }

        Ok(prior_values)
    }

    /// Apply the action after prior values have been safely journaled (SAF-04).
    pub fn apply_with_priors(
        &self,
        intent: &RemediationIntent,
        _prior_values: &HashMap<String, String>,
    ) -> Result<()> {
        let target_dir = self.resolve_path(&intent.target_cgroup);
        if !target_dir.exists() {
            return Err(TsukiError::Validation(format!(
                "Target cgroup directory '{:?}' does not exist",
                target_dir
            )));
        }

        match intent.template {
            ActionTemplate::ThrottleMemory => {
                let mem_high_file = target_dir.join("memory.high");
                let mem_curr_file = target_dir.join("memory.current");

                let target_bytes = if let Some(ratio) = intent
                    .params
                    .get("ratio_of_current")
                    .and_then(|v| v.as_f64())
                {
                    let current_bytes = self
                        .read_prior_value(&mem_curr_file)
                        .ok()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(128 * 1024 * 1024);
                    ((current_bytes as f64) * ratio).round() as u64
                } else if let Some(bytes) = intent
                    .params
                    .get("memory_high_bytes")
                    .and_then(|v| v.as_u64())
                {
                    bytes
                } else {
                    return Err(TsukiError::Validation(
                        "Missing memory throttle parameter".into(),
                    ));
                };

                info!(
                    "Applying memory.high throttle on {:?} -> {} bytes",
                    target_dir, target_bytes
                );
                fs::write(&mem_high_file, target_bytes.to_string())?;
            }
            ActionTemplate::ReweightCpu => {
                let cpu_weight_file = target_dir.join("cpu.weight");
                let weight = intent
                    .params
                    .get("cpu_weight")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| TsukiError::Validation("Missing cpu_weight parameter".into()))?;

                info!(
                    "Applying cpu.weight reweight on {:?} -> {}",
                    target_dir, weight
                );
                fs::write(&cpu_weight_file, weight.to_string())?;
            }
            ActionTemplate::FreezeCgroup => {
                let freeze_file = target_dir.join("cgroup.freeze");
                info!("Freezing cgroup {:?}", target_dir);
                fs::write(&freeze_file, "1")?;
            }
        }

        Ok(())
    }

    /// Safely apply a validated intent and return prior values for rollback journaling.
    pub fn apply(&self, intent: &RemediationIntent) -> Result<HashMap<String, String>> {
        let prior_values = self.read_prior_values(intent)?;
        self.apply_with_priors(intent, &prior_values)?;
        Ok(prior_values)
    }

    /// Revert a previously applied action using stored prior values (SAF-04).
    pub fn rollback(
        &self,
        target_cgroup: &str,
        prior_values: &HashMap<String, String>,
    ) -> Result<()> {
        let target_dir = self.resolve_path(target_cgroup);
        if !target_dir.exists() {
            warn!(
                "Target cgroup {:?} already removed during rollback; skipping.",
                target_dir
            );
            return Ok(());
        }

        info!(
            "Rolling back cgroup {:?} to prior state: {:?}",
            target_dir, prior_values
        );
        let mut failures = Vec::new();
        for (filename, prior_val) in prior_values {
            let file_path = target_dir.join(filename);
            if file_path.exists() {
                if let Err(e) = fs::write(&file_path, prior_val) {
                    warn!("Failed to write prior value to {:?}: {}", file_path, e);
                    failures.push(format!("{}: {}", filename, e));
                }
            }
        }

        if !failures.is_empty() {
            return Err(TsukiError::Io(io::Error::other(format!(
                "Partial rollback failures: {}",
                failures.join(", ")
            ))));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_and_rollback_mock_cgroup() {
        let temp_dir = std::env::temp_dir().join("tsuki_mock_cgroup");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Initialize mock cgroup files
        fs::write(temp_dir.join("memory.current"), "100000000").unwrap();
        fs::write(temp_dir.join("memory.high"), "max").unwrap();
        fs::write(temp_dir.join("cpu.weight"), "100").unwrap();

        let executor = ActionExecutor::new(std::env::temp_dir());

        let mut params = HashMap::new();
        params.insert("ratio_of_current".into(), serde_json::json!(0.80));

        let intent = RemediationIntent {
            intent_id: "int-mock-01".into(),
            template: ActionTemplate::ThrottleMemory,
            target_cgroup: "/tsuki_mock_cgroup".into(),
            params,
            rationale: "testing".into(),
            evidence_ids: vec![],
            ttl_seconds: 60,
        };

        // Apply
        let prior = executor.apply(&intent).unwrap();
        assert_eq!(prior.get("memory.high").unwrap(), "max");

        // Verify applied value
        let current_val = fs::read_to_string(temp_dir.join("memory.high")).unwrap();
        assert_eq!(current_val.trim(), "80000000");

        // Rollback
        executor.rollback(&intent.target_cgroup, &prior).unwrap();
        let restored_val = fs::read_to_string(temp_dir.join("memory.high")).unwrap();
        assert_eq!(restored_val.trim(), "max");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
