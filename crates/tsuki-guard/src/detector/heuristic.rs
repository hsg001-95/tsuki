use crate::features::FeatureVector;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub anomaly_score: f32,
    pub is_alert: bool,
    pub fault_type: String,
    pub predicted_lead_time_sec: Option<f32>,
    pub rationale: String,
}

/// Baseline heuristic detector for early warning before machine freeze.
#[derive(Debug, Clone)]
pub struct HeuristicDetector {
    pub memory_some_threshold: f64,
    pub memory_full_threshold: f64,
    pub cpu_some_threshold: f64,
    pub io_full_threshold: f64,
    pub refault_rate_threshold: u64,
}

impl Default for HeuristicDetector {
    fn default() -> Self {
        Self {
            memory_some_threshold: 20.0,
            memory_full_threshold: 10.0,
            cpu_some_threshold: 60.0,
            io_full_threshold: 25.0,
            refault_rate_threshold: 1000,
        }
    }
}

impl HeuristicDetector {
    pub fn evaluate(&self, features: &FeatureVector) -> DetectionResult {
        let mut score = 0.0f32;
        let mut fault_type = "healthy".to_string();
        let mut rationale = String::new();
        let mut lead_time = None;

        // 1. Memory Pressure Evaluation
        if features.psi_mem_full_avg10 > self.memory_full_threshold {
            score = (score + 0.6).min(1.0);
            fault_type = "memory_thrash".to_string();
            rationale = format!(
                "Severe memory full stall: {:.1}% (threshold: {:.1}%)",
                features.psi_mem_full_avg10, self.memory_full_threshold
            );
            lead_time = Some(15.0);
        } else if features.psi_mem_some_avg10 > self.memory_some_threshold {
            score = (score + 0.4).min(1.0);
            fault_type = "memory_pressure".to_string();
            rationale = format!(
                "Elevated memory some stall: {:.1}% (threshold: {:.1}%)",
                features.psi_mem_some_avg10, self.memory_some_threshold
            );
            lead_time = Some(25.0);
        }

        // Refault / Paging acceleration
        if features.refault_rate > self.refault_rate_threshold {
            score = (score + 0.25).min(1.0);
            if fault_type == "healthy" {
                fault_type = "memory_thrash".to_string();
            }
            if !rationale.is_empty() {
                rationale.push_str("; ");
            }
            rationale.push_str(&format!(
                "High workingset refault rate: {}/s",
                features.refault_rate
            ));
        }

        // Explosive trend check (delta > 2%/sec)
        if features.psi_mem_some_delta > 2.0 {
            score = (score + 0.2).min(1.0);
            lead_time = lead_time
                .map(|t: f32| (t * 0.7f32).max(5.0f32))
                .or(Some(10.0f32));
        }

        // 2. CPU Contention Evaluation
        if features.psi_cpu_some_avg10 > self.cpu_some_threshold {
            let cpu_score = ((features.psi_cpu_some_avg10 / 100.0) as f32).min(1.0);
            if cpu_score > score {
                score = cpu_score;
                fault_type = "cpu_contention".to_string();
                rationale = format!(
                    "Severe CPU run-queue stall: {:.1}%",
                    features.psi_cpu_some_avg10
                );
                lead_time = Some(20.0);
            }
        }

        // 3. I/O Saturation Evaluation
        if features.psi_io_full_avg10 > self.io_full_threshold {
            let io_score = ((features.psi_io_full_avg10 / 100.0) as f32).min(1.0);
            if io_score > score {
                score = io_score;
                fault_type = "io_saturation".to_string();
                rationale = format!(
                    "Severe block I/O full stall: {:.1}%",
                    features.psi_io_full_avg10
                );
                lead_time = Some(15.0);
            }
        }

        DetectionResult {
            anomaly_score: score,
            is_alert: score >= 0.5,
            fault_type,
            predicted_lead_time_sec: lead_time,
            rationale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_evaluation() {
        let detector = HeuristicDetector::default();
        let features = FeatureVector::default();
        let res = detector.evaluate(&features);

        assert_eq!(res.anomaly_score, 0.0);
        assert!(!res.is_alert);
        assert_eq!(res.fault_type, "healthy");
    }

    #[test]
    fn test_memory_thrash_alert() {
        let detector = HeuristicDetector::default();
        let features = FeatureVector {
            psi_mem_some_avg10: 35.0,
            psi_mem_full_avg10: 18.0,
            refault_rate: 2500,
            psi_mem_some_delta: 3.5,
            ..Default::default()
        };
        let res = detector.evaluate(&features);

        assert!(res.anomaly_score >= 0.7);
        assert!(res.is_alert);
        assert_eq!(res.fault_type, "memory_thrash");
        assert!(res.predicted_lead_time_sec.is_some());
    }
}
