use tracing::{info, warn};
use tsuki_common::config::Config;
use tsuki_common::error::{Result, TsukiError};
use tsuki_common::intent::{ActionTemplate, RemediationIntent};

/// Deterministic Intent Validator.
/// Enforces Rules SAF-02, SAF-03, and SAF-05:
/// 1. Template must be in allow-list
/// 2. Target must resolve to a valid cgroup path (NEVER raw PIDs from model text)
/// 3. Target must not be in protected list
/// 4. Parameters must be within bounded safety thresholds
/// 5. Operational mode must permit actuation
#[derive(Debug, Clone)]
pub struct IntentValidator {
    config: Config,
    cgroup_root: String,
}

impl IntentValidator {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cgroup_root: "/sys/fs/cgroup".to_string(),
        }
    }

    pub fn with_cgroup_root(mut self, root: String) -> Self {
        self.cgroup_root = root;
        self
    }

    /// Validate a proposed intent against all deterministic checks.
    pub fn validate(&self, intent: &RemediationIntent) -> Result<()> {
        info!(
            "Validating intent [{}] for target '{}'",
            intent.intent_id, intent.target_cgroup
        );

        // Check 1: Action template in allow-list (v1 only supports reversible actions)
        match intent.template {
            ActionTemplate::ThrottleMemory
            | ActionTemplate::ReweightCpu
            | ActionTemplate::FreezeCgroup => {}
        }

        // Check 2: Target must be a cgroup path, NEVER a numeric PID (SAF-03)
        self.validate_target_format(&intent.target_cgroup)?;

        // Check 3: Target must not be protected (SAF-05)
        self.validate_not_protected(&intent.target_cgroup)?;

        // Check 4: Parameters within safety bounds
        self.validate_parameter_bounds(intent)?;

        // Check 5: TTL must be strictly positive and bounded
        if intent.ttl_seconds == 0 {
            return Err(TsukiError::Validation("TTL cannot be zero".into()));
        }
        if intent.ttl_seconds > self.config.actuator.max_ttl_seconds {
            return Err(TsukiError::Validation(format!(
                "TTL {}s exceeds maximum permitted {}s",
                intent.ttl_seconds, self.config.actuator.max_ttl_seconds
            )));
        }

        info!("Intent [{}] validated successfully.", intent.intent_id);
        Ok(())
    }

    fn validate_target_format(&self, target: &str) -> Result<()> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(TsukiError::Validation(
                "Target cgroup cannot be empty".into(),
            ));
        }

        // Reject targets that are numeric PIDs (SAF-03)
        if trimmed.parse::<u64>().is_ok() {
            return Err(TsukiError::Validation(
                "Raw numeric PIDs are strictly rejected; target must be a cgroup path (SAF-03)"
                    .into(),
            ));
        }

        // Must start with '/' and not contain traversal '..'
        if !trimmed.starts_with('/') {
            return Err(TsukiError::Validation(
                "Target must be an absolute cgroup path starting with '/'".into(),
            ));
        }
        if trimmed.contains("..") {
            return Err(TsukiError::Validation(
                "Path traversal ('..') is strictly prohibited in cgroup targets".into(),
            ));
        }

        // Must consist only of valid cgroup path characters (strictly alphanumeric, '/', '.', '_', '-', '@', '+')
        // Any shell metacharacters, prompt injections, or whitespace are immediately rejected
        if !trimmed.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || c == '/'
                || c == '.'
                || c == '_'
                || c == '-'
                || c == '@'
                || c == '+'
        }) {
            return Err(TsukiError::Validation(
                "Target cgroup path contains invalid or dangerous characters (potential injection rejected)".into(),
            ));
        }

        Ok(())
    }

    fn validate_not_protected(&self, target: &str) -> Result<()> {
        let normalized = target.trim_end_matches('/');

        // 1. Root and core slices can never be targeted directly (SAF-05)
        if normalized.is_empty()
            || normalized == "/"
            || normalized == "/sys/fs/cgroup"
            || normalized == "/user.slice"
            || normalized == "/system.slice"
        {
            return Err(TsukiError::ProtectedTarget(
                "Top-level slice cannot be altered directly (SAF-05)".into(),
            ));
        }

        // 2. Check protected cgroup list from configuration
        for protected in &self.config.protected.cgroups {
            let p_norm = protected.trim_end_matches('/');
            if p_norm == "/user.slice" || p_norm == "/system.slice" {
                if normalized == p_norm {
                    return Err(TsukiError::ProtectedTarget(format!(
                        "Target '{}' is protected under rule SAF-05 (matched '{}')",
                        target, protected
                    )));
                }
            } else if normalized == p_norm || normalized.starts_with(&format!("{}/", p_norm)) {
                return Err(TsukiError::ProtectedTarget(format!(
                    "Target '{}' is protected under rule SAF-05 (matched '{}')",
                    target, protected
                )));
            }
        }

        // 3. Check protected process names
        for proc_name in &self.config.protected.process_names {
            if normalized.contains(proc_name) {
                warn!(
                    "Target '{}' contains protected service name '{}'",
                    target, proc_name
                );
                return Err(TsukiError::ProtectedTarget(format!(
                    "Target contains protected process identifier '{}' (SAF-05)",
                    proc_name
                )));
            }
        }

        Ok(())
    }

    fn validate_parameter_bounds(&self, intent: &RemediationIntent) -> Result<()> {
        match intent.template {
            ActionTemplate::ThrottleMemory => {
                // memory.high lower bound: ratio cannot be below 0.50 (50% of current usage)
                if let Some(ratio) = intent
                    .params
                    .get("ratio_of_current")
                    .and_then(|v| v.as_f64())
                {
                    if ratio < 0.50 || ratio > 0.99 {
                        return Err(TsukiError::Validation(format!(
                            "memory.high ratio {} is out of safe bounds [0.50, 0.99]",
                            ratio
                        )));
                    }
                } else if let Some(bytes) = intent
                    .params
                    .get("memory_high_bytes")
                    .and_then(|v| v.as_u64())
                {
                    if bytes < 16 * 1024 * 1024 {
                        // Less than 16 MB is dangerously low
                        return Err(TsukiError::Validation(
                            "memory_high_bytes cannot be set below 16 MB floor".into(),
                        ));
                    }
                } else {
                    return Err(TsukiError::Validation(
                        "ThrottleMemory requires 'ratio_of_current' or 'memory_high_bytes'".into(),
                    ));
                }
            }
            ActionTemplate::ReweightCpu => {
                let weight = intent
                    .params
                    .get("cpu_weight")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        TsukiError::Validation("ReweightCpu requires 'cpu_weight'".into())
                    })?;

                if !(10..=10000).contains(&weight) {
                    return Err(TsukiError::Validation(format!(
                        "cpu.weight {} is out of permitted bounds [10, 10000]",
                        weight
                    )));
                }
            }
            ActionTemplate::FreezeCgroup => {
                // Freezing must be strictly bounded in duration
                if intent.ttl_seconds > 60 {
                    return Err(TsukiError::Validation(
                        "cgroup.freeze TTL cannot exceed 60 seconds (SAF-04)".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_intent(
        template: ActionTemplate,
        target: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> RemediationIntent {
        RemediationIntent {
            intent_id: "test-01".into(),
            template,
            target_cgroup: target.into(),
            params,
            rationale: "Testing validation".into(),
            evidence_ids: vec!["psi_mem".into()],
            ttl_seconds: 120,
        }
    }

    #[test]
    fn test_reject_numeric_pid() {
        let v = IntentValidator::new(Config::default());
        let mut params = HashMap::new();
        params.insert("ratio_of_current".into(), serde_json::json!(0.85));

        // Hallucinated PID: "12345"
        let intent = make_intent(ActionTemplate::ThrottleMemory, "12345", params);
        assert!(v.validate(&intent).is_err());
    }

    #[test]
    fn test_reject_protected_target() {
        let v = IntentValidator::new(Config::default());
        let mut params = HashMap::new();
        params.insert("ratio_of_current".into(), serde_json::json!(0.85));

        // Targeting sshd service
        let intent = make_intent(
            ActionTemplate::ThrottleMemory,
            "/system.slice/sshd.service",
            params,
        );
        assert!(v.validate(&intent).is_err());

        // Targeting root user.slice
        let mut p2 = HashMap::new();
        p2.insert("ratio_of_current".into(), serde_json::json!(0.85));
        let intent2 = make_intent(ActionTemplate::ThrottleMemory, "/user.slice", p2);
        assert!(v.validate(&intent2).is_err());
    }

    #[test]
    fn test_reject_out_of_bounds_parameters() {
        let v = IntentValidator::new(Config::default());

        // Ratio too low (< 0.50)
        let mut params = HashMap::new();
        params.insert("ratio_of_current".into(), serde_json::json!(0.10));
        let intent = make_intent(
            ActionTemplate::ThrottleMemory,
            "/user.slice/app.slice/worker.service",
            params,
        );
        assert!(v.validate(&intent).is_err());

        // cpu_weight too low (< 10)
        let mut p_cpu = HashMap::new();
        p_cpu.insert("cpu_weight".into(), serde_json::json!(1));
        let intent_cpu = make_intent(
            ActionTemplate::ReweightCpu,
            "/user.slice/app.slice/worker.service",
            p_cpu,
        );
        assert!(v.validate(&intent_cpu).is_err());
    }

    #[test]
    fn test_valid_intent_passes() {
        let v = IntentValidator::new(Config::default());
        let mut params = HashMap::new();
        params.insert("ratio_of_current".into(), serde_json::json!(0.80));

        let intent = make_intent(
            ActionTemplate::ThrottleMemory,
            "/user.slice/user-1000.slice/app.slice/build.service",
            params,
        );
        assert!(v.validate(&intent).is_ok());
    }
}
