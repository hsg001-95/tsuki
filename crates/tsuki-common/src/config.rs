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
    pub fn load_from_str(s: &str) -> Result<Self> {
        toml::from_str(s).map_err(|e| TsukiError::Config(e.to_string()))
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }
}
