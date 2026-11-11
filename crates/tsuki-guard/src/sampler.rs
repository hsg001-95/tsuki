use crate::collector::Collector;
use crate::detector::{DetectionResult, HeuristicDetector};
use crate::features::{FeaturePipeline, FeatureVector, SlidingWindow};
use crate::snapshot::SnapshotRingBuffer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tsuki_common::config::GuardConfig;

pub struct Sampler {
    config: GuardConfig,
    collector: Collector,
    window: SlidingWindow,
    detector: HeuristicDetector,
    ring_buffer: SnapshotRingBuffer,
    running: Arc<AtomicBool>,
    last_alert_time: Option<Instant>,
}

impl Sampler {
    pub fn new(
        config: GuardConfig,
        collector: Collector,
        ring_buffer: SnapshotRingBuffer,
        running: Arc<AtomicBool>,
    ) -> Self {
        let capacity = config.window_size_samples.max(10);
        Self {
            config,
            collector,
            window: SlidingWindow::new(capacity),
            detector: HeuristicDetector::default(),
            ring_buffer,
            running,
            last_alert_time: None,
        }
    }

    /// Run a single sampling iteration. Returns the extracted features and detection result.
    pub fn step(&mut self) -> Result<(FeatureVector, DetectionResult), std::io::Error> {
        let start = Instant::now();

        // 1. Collect telemetry
        let sample = self.collector.collect_sample()?;

        // 2. Push to circular window
        self.window.push(sample);

        // 3. Extract features
        let features = FeaturePipeline::extract(&self.window);

        // 4. Evaluate detection
        let detection = self.detector.evaluate(&features);

        // Check if decision latency exceeds budget
        let elapsed = start.elapsed();
        if elapsed.as_millis() > self.config.inference_p99_budget_ms as u128 {
            warn!(
                "Sampling step latency ({:?}) exceeded budget ({}ms)",
                elapsed, self.config.inference_p99_budget_ms
            );
        }

        // 5. Handle Alert & Snapshot generation
        if detection.is_alert {
            let should_write = match self.last_alert_time {
                Some(last) => last.elapsed() > Duration::from_secs(5), // rate-limit snapshots to once every 5s
                None => true,
            };

            if should_write {
                self.last_alert_time = Some(Instant::now());
                info!(
                    "🚨 EARLY WARNING ALERT: [{}] score={:.2} lead_time={:?}s: {}",
                    detection.fault_type,
                    detection.anomaly_score,
                    detection.predicted_lead_time_sec,
                    detection.rationale
                );

                let ts = self.window.latest().map(|s| s.timestamp_ms).unwrap_or(0);
                let snapshot_id = format!("snap-{}-{:.0}", ts, detection.anomaly_score * 100.0);

                let mut metadata = HashMap::new();
                metadata.insert("fault_type".to_string(), detection.fault_type.clone());
                metadata.insert("rationale".to_string(), detection.rationale.clone());

                let snapshot = tsuki_common::telemetry::EvidenceSnapshot {
                    snapshot_id,
                    timestamp_ms: ts,
                    anomaly_score: detection.anomaly_score,
                    predicted_lead_time_sec: detection.predicted_lead_time_sec,
                    samples: self.window.samples(),
                    primary_cgroup: features.primary_cgroup.clone(),
                    metadata,
                };

                let _ = self.ring_buffer.write_snapshot(&snapshot);
            }
        }

        Ok((features, detection))
    }

    /// Calculate adaptive sleep duration based on current anomaly score.
    pub fn calculate_sleep_interval(&self, anomaly_score: f32) -> Duration {
        if !self.config.adaptive_sampling || anomaly_score < 0.3 {
            // Normal baseline 1 Hz
            let hz = self.config.sampling_rate_hz.max(0.1);
            Duration::from_secs_f64(1.0 / hz)
        } else {
            // Adaptive ramp-up between baseline and max_adaptive_hz
            let t = ((anomaly_score - 0.3) / 0.7).clamp(0.0, 1.0) as f64;
            let target_hz = self.config.sampling_rate_hz
                + t * (self.config.max_adaptive_hz - self.config.sampling_rate_hz);
            Duration::from_secs_f64(1.0 / target_hz)
        }
    }

    /// Main loop for the guard daemon.
    pub fn run(&mut self) -> Result<(), std::io::Error> {
        info!(
            "Guard sampling loop running (base rate: {} Hz)",
            self.config.sampling_rate_hz
        );

        while self.running.load(Ordering::Relaxed) {
            let step_start = Instant::now();

            match self.step() {
                Ok((_features, detection)) => {
                    let sleep_dur = self.calculate_sleep_interval(detection.anomaly_score);
                    let elapsed = step_start.elapsed();

                    if elapsed < sleep_dur {
                        std::thread::sleep(sleep_dur - elapsed);
                    }
                }
                Err(e) => {
                    warn!("Error during telemetry collection step: {}", e);
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }

        info!("Guard sampling loop shutdown cleanly.");
        Ok(())
    }
}
