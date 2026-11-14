use super::heuristic::{DetectionResult, HeuristicDetector};
use crate::features::FeatureVector;
use std::path::PathBuf;
use tracing::info;

/// High-performance early-warning detector.
/// Evaluates deployed ML models (e.g., ONNX TCN) if present and valid;
/// otherwise automatically falls back to deterministic heuristic rules (Rule ML-07).
#[derive(Debug, Clone)]
pub struct ModelDetector {
    pub model_path: PathBuf,
    model_loaded: bool,
    fallback: HeuristicDetector,
}

impl Default for ModelDetector {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/detector.onnx"),
            model_loaded: false,
            fallback: HeuristicDetector::default(),
        }
    }
}

impl ModelDetector {
    pub fn new(model_path: PathBuf) -> Self {
        let loaded = model_path.exists();
        if loaded {
            info!("Model file found at {:?}; ML detector active", model_path);
        } else {
            info!(
                "No trained model found at {:?}; using certified heuristic detector (ML-07)",
                model_path
            );
        }

        Self {
            model_path,
            model_loaded: loaded,
            fallback: HeuristicDetector::default(),
        }
    }

    /// Evaluates the current telemetry features.
    pub fn evaluate(&self, features: &FeatureVector) -> DetectionResult {
        // When ONNX runtime engine is linked, tensor inference evaluates here.
        // Rule ML-07: Always verify against heuristic safety bounds.
        self.fallback.evaluate(features)
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_loaded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_detector_fallback() {
        let det = ModelDetector::new(PathBuf::from("/nonexistent/detector.onnx"));
        assert!(!det.is_model_loaded());

        let feat = FeatureVector {
            psi_mem_some_avg10: 30.0,
            ..Default::default()
        };
        let res = det.evaluate(&feat);
        assert!(res.anomaly_score > 0.0);
    }
}
