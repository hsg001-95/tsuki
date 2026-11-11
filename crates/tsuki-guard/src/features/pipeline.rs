use super::window::SlidingWindow;
use serde::{Deserialize, Serialize};

/// Summary features extracted from a sliding window for early-warning detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureVector {
    // Current instantaneous PSI averages
    pub psi_mem_some_avg10: f64,
    pub psi_mem_full_avg10: f64,
    pub psi_cpu_some_avg10: f64,
    pub psi_io_some_avg10: f64,
    pub psi_io_full_avg10: f64,

    // Trend (rate of change over the last ~5-10 samples)
    pub psi_mem_some_delta: f64,
    pub psi_mem_full_delta: f64,
    pub psi_cpu_some_delta: f64,

    // Moving averages over available window
    pub window_mean_mem_some: f64,
    pub window_mean_cpu_some: f64,
    pub window_mean_io_some: f64,

    // vmstat rates
    pub refault_rate: u64,
    pub pgmajfault_rate: u64,
    pub pgscan_direct_rate: u64,
    pub compact_stall_rate: u64,
    pub oom_kill_rate: u64,

    // Primary cgroup under highest resource pressure
    pub primary_cgroup: Option<String>,
    pub primary_cgroup_memory_bytes: u64,
}

pub struct FeaturePipeline;

impl FeaturePipeline {
    /// Extract structured features from the sliding window.
    pub fn extract(window: &SlidingWindow) -> FeatureVector {
        if window.is_empty() {
            return FeatureVector::default();
        }

        let latest = match window.latest() {
            Some(s) => s,
            None => return FeatureVector::default(),
        };

        let mut sum_mem_some = 0.0;
        let mut sum_cpu_some = 0.0;
        let mut sum_io_some = 0.0;
        let n = window.len() as f64;

        for s in window.iter() {
            sum_mem_some += s.pressure.memory.some_avg10;
            sum_cpu_some += s.pressure.cpu.some_avg10;
            sum_io_some += s.pressure.io.some_avg10;
        }

        // Calculate delta against sample ~5-10 positions back (or oldest if smaller)
        let samples: Vec<_> = window.iter().collect();
        let lookback_idx = if samples.len() > 5 {
            samples.len() - 5
        } else {
            0
        };

        let past = samples[lookback_idx];
        let delta_sec = (latest.timestamp_ms.saturating_sub(past.timestamp_ms) as f64) / 1000.0;

        let compute_delta = |curr: f64, prev: f64| -> f64 {
            if delta_sec > 0.0 {
                (curr - prev) / delta_sec
            } else {
                0.0
            }
        };

        let psi_mem_some_delta = compute_delta(
            latest.pressure.memory.some_avg10,
            past.pressure.memory.some_avg10,
        );
        let psi_mem_full_delta = compute_delta(
            latest.pressure.memory.full_avg10,
            past.pressure.memory.full_avg10,
        );
        let psi_cpu_some_delta =
            compute_delta(latest.pressure.cpu.some_avg10, past.pressure.cpu.some_avg10);

        // Find primary culprit cgroup
        let mut primary_cgroup = None;
        let mut primary_cgroup_memory_bytes = 0;
        if let Some(top) = latest.cgroups.first() {
            primary_cgroup = Some(top.cgroup_path.clone());
            primary_cgroup_memory_bytes = top.memory_current_bytes;
        }

        FeatureVector {
            psi_mem_some_avg10: latest.pressure.memory.some_avg10,
            psi_mem_full_avg10: latest.pressure.memory.full_avg10,
            psi_cpu_some_avg10: latest.pressure.cpu.some_avg10,
            psi_io_some_avg10: latest.pressure.io.some_avg10,
            psi_io_full_avg10: latest.pressure.io.full_avg10,

            psi_mem_some_delta,
            psi_mem_full_delta,
            psi_cpu_some_delta,

            window_mean_mem_some: sum_mem_some / n,
            window_mean_cpu_some: sum_cpu_some / n,
            window_mean_io_some: sum_io_some / n,

            refault_rate: latest.vmstat.workingset_refault,
            pgmajfault_rate: latest.vmstat.pgmajfault,
            pgscan_direct_rate: latest.vmstat.pgscan_direct,
            compact_stall_rate: latest.vmstat.compact_stall,
            oom_kill_rate: latest.vmstat.oom_kill,

            primary_cgroup,
            primary_cgroup_memory_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsuki_common::telemetry::{PressureRecord, PsiMetrics, TelemetrySample};

    #[test]
    fn test_feature_extraction() {
        let mut window = SlidingWindow::new(10);
        let sample = TelemetrySample {
            timestamp_ms: 1000,
            pressure: PressureRecord {
                memory: PsiMetrics {
                    some_avg10: 12.5,
                    full_avg10: 5.0,
                    ..Default::default()
                },
                cpu: PsiMetrics {
                    some_avg10: 45.0,
                    ..Default::default()
                },
                io: PsiMetrics::default(),
            },
            vmstat: Default::default(),
            cgroups: vec![],
        };

        window.push(sample);
        let feat = FeaturePipeline::extract(&window);

        assert_eq!(feat.psi_mem_some_avg10, 12.5);
        assert_eq!(feat.psi_mem_full_avg10, 5.0);
        assert_eq!(feat.psi_cpu_some_avg10, 45.0);
        assert_eq!(feat.window_mean_mem_some, 12.5);
    }
}
