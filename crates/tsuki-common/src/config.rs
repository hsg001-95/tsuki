use crate::error::{Result, TsukiError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub guard: GuardConfig,
    #[serde(default)]
    pub actuator: ActuatorConfig,
    #[serde(default)]
    pub protected: ProtectedConfig,
    #[serde(default)]
    pub reasoning: ReasoningConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub mode: String, // "shadow", "approval", "autonomous"
    pub log_level: String,
    pub snapshot_dir: PathBuf,
    pub journal_path: PathBuf,
    pub socket_path: PathBuf,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            mode: "shadow".into(),
            log_level: "info".into(),
            snapshot_dir: PathBuf::from("/var/run/tsuki/snapshots"),
            journal_path: PathBuf::from("/var/lib/tsuki/rollback.journal"),
            socket_path: PathBuf::from("/var/run/tsuki/actuator.sock"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfig {
    pub sampling_rate_hz: f64,
    pub adaptive_sampling: bool,
    pub max_adaptive_hz: f64,
    pub window_size_samples: usize,
    pub inference_p99_budget_ms: u64,
    pub max_rss_mb: u64,
    pub enable_ebpf: bool,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self {
            sampling_rate_hz: 1.0,
            adaptive_sampling: true,
            max_adaptive_hz: 10.0,
            window_size_samples: 60,
            inference_p99_budget_ms: 5,
            max_rss_mb: 100,
            enable_ebpf: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorConfig {
    pub max_decision_latency_ms: u64,
    pub default_ttl_seconds: u64,
    pub max_ttl_seconds: u64,
    pub cooldown_seconds: u64,
}

impl Default for ActuatorConfig {
    fn default() -> Self {
        Self {
            max_decision_latency_ms: 50,
            default_ttl_seconds: 300,
            max_ttl_seconds: 1800,
            cooldown_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedConfig {
    pub cgroups: Vec<String>,
    pub process_names: Vec<String>,
}

impl Default for ProtectedConfig {
    fn default() -> Self {
        Self {
            cgroups: vec![
                "/system.slice/sshd.service".into(),
                "/system.slice/dbus.service".into(),
                "/system.slice/systemd-journald.service".into(),
                "/user.slice".into(),
            ],
            process_names: vec![
                "systemd".into(),
                "sshd".into(),
                "Xorg".into(),
                "wayland".into(),
                "gnome-shell".into(),
                "dbus-daemon".into(),
                "tsuki-guard".into(),
                "tsuki-actuator".into(),
                "tsuki-rollbackd".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub local_llm_url: String,
    pub local_llm_model: String,
    pub rag_index_path: PathBuf,
    pub max_rss_mb: u64,
    pub timeout_seconds: u64,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            local_llm_url: "http://127.0.0.1:11434".into(),
            local_llm_model: "llama3.1:8b".into(),
            rag_index_path: PathBuf::from("/var/lib/tsuki/rag_index"),
            max_rss_mb: 4096,
            timeout_seconds: 15,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        let valid_modes = ["shadow", "approval", "autonomous"];
        if !valid_modes.contains(&self.general.mode.as_str()) {
            return Err(TsukiError::Config(format!(
                "Invalid mode '{}', expected one of {:?}",
                self.general.mode, valid_modes
            )));
        }

        if self.guard.sampling_rate_hz <= 0.0 {
            return Err(TsukiError::Config("sampling_rate_hz must be > 0.0".into()));
        }
        if self.guard.adaptive_sampling && self.guard.max_adaptive_hz < self.guard.sampling_rate_hz {
            return Err(TsukiError::Config("max_adaptive_hz cannot be less than sampling_rate_hz".into()));
        }
        if self.guard.window_size_samples < 5 {
            return Err(TsukiError::Config("window_size_samples must be at least 5".into()));
        }

        if self.actuator.default_ttl_seconds == 0 {
            return Err(TsukiError::Config("default_ttl_seconds must be > 0".into()));
        }
        if self.actuator.max_ttl_seconds < self.actuator.default_ttl_seconds {
            return Err(TsukiError::Config("max_ttl_seconds cannot be less than default_ttl_seconds".into()));
        }
        if self.actuator.cooldown_seconds == 0 {
            return Err(TsukiError::Config("cooldown_seconds must be > 0".into()));
        }

        Ok(())
    }

    pub fn load_from_str(s: &str) -> Result<Self> {
        let cfg: Self = toml::from_str(s).map_err(|e| TsukiError::Config(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }
}
