use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PsiMetrics {
    pub some_avg10: f64,
    pub some_avg60: f64,
    pub some_avg300: f64,
    pub some_total_us: u64,
    pub full_avg10: f64,
    pub full_avg60: f64,
    pub full_avg300: f64,
    pub full_total_us: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressureRecord {
    pub cpu: PsiMetrics,
    pub memory: PsiMetrics,
    pub io: PsiMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmstatRecord {
    pub pgpgin: u64,
    pub pgpgout: u64,
    pub pswpin: u64,
    pub pswpout: u64,
    pub pgfault: u64,
    pub pgmajfault: u64,
    pub workingset_refault: u64,
    pub pgscan_kswapd: u64,
    pub pgscan_direct: u64,
    pub pgsteal_kswapd: u64,
    pub pgsteal_direct: u64,
    pub compact_stall: u64,
    pub oom_kill: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CgroupStatRecord {
    pub cgroup_path: String,
    pub memory_current_bytes: u64,
    pub memory_high_bytes: Option<u64>,
    pub memory_max_bytes: Option<u64>,
    pub cpu_usage_usec: u64,
    pub cpu_weight: Option<u64>,
    pub io_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub timestamp_ms: u64,
    pub pressure: PressureRecord,
    pub vmstat: VmstatRecord,
    pub cgroups: Vec<CgroupStatRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSnapshot {
    pub snapshot_id: String,
    pub timestamp_ms: u64,
    pub anomaly_score: f32,
    pub predicted_lead_time_sec: Option<f32>,
    pub samples: Vec<TelemetrySample>,
    pub primary_cgroup: Option<String>,
    pub metadata: HashMap<String, String>,
}
