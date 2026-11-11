use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{debug, warn};
use tsuki_common::telemetry::EvidenceSnapshot;

/// Bounded ring-buffer directory writer for EvidenceSnapshots.
#[derive(Debug, Clone)]
pub struct SnapshotRingBuffer {
    output_dir: PathBuf,
    max_snapshots: usize,
}

impl Default for SnapshotRingBuffer {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("/var/run/tsuki/snapshots"),
            max_snapshots: 20,
        }
    }
}

impl SnapshotRingBuffer {
    pub fn new(output_dir: PathBuf, max_snapshots: usize) -> Self {
        Self {
            output_dir,
            max_snapshots,
        }
    }

    /// Ensure output directory exists.
    pub fn init(&self) -> io::Result<()> {
        fs::create_dir_all(&self.output_dir)
    }

    /// Write an EvidenceSnapshot atomically to disk.
    pub fn write_snapshot(&self, snapshot: &EvidenceSnapshot) -> io::Result<PathBuf> {
        self.init()?;

        let filename = format!("{}.json", snapshot.snapshot_id);
        let final_path = self.output_dir.join(&filename);
        let temp_path = self
            .output_dir
            .join(format!("{}.tmp", snapshot.snapshot_id));

        // Atomic write: write to temp file then rename
        {
            let mut file = File::create(&temp_path)?;
            let json_bytes = serde_json::to_vec_pretty(snapshot)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            file.write_all(&json_bytes)?;
            file.sync_all()?;
        }

        debug!("Wrote evidence snapshot to {:?}", final_path);
        fs::rename(&temp_path, &final_path)?;

        // Prune older snapshots if count exceeds limit
        self.prune_old_snapshots();

        Ok(final_path)
    }

    fn prune_old_snapshots(&self) {
        let entries = match fs::read_dir(&self.output_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mod_time) = meta.modified() {
                        files.push((path, mod_time));
                    }
                }
            }
        }

        if files.len() > self.max_snapshots {
            // Sort oldest first
            files.sort_by_key(|(_, time)| *time);
            let to_remove = files.len() - self.max_snapshots;
            for (path, _) in files.iter().take(to_remove) {
                if let Err(e) = fs::remove_file(path) {
                    warn!("Failed to prune old snapshot {:?}: {}", path, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_ring_buffer_atomic_write_and_prune() {
        let temp_dir = std::env::temp_dir().join("tsuki_test_snapshots");
        let _ = fs::remove_dir_all(&temp_dir);

        let ring = SnapshotRingBuffer::new(temp_dir.clone(), 3);
        ring.init().unwrap();

        for i in 1..=5 {
            let snapshot = EvidenceSnapshot {
                snapshot_id: format!("snap-{}", i),
                timestamp_ms: i * 1000,
                anomaly_score: 0.8,
                predicted_lead_time_sec: Some(15.0),
                samples: vec![],
                primary_cgroup: Some("/system.slice/test.service".into()),
                metadata: HashMap::new(),
            };
            let res = ring.write_snapshot(&snapshot);
            assert!(res.is_ok());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Count remaining JSON files
        let entries: Vec<_> = fs::read_dir(&temp_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .collect();

        assert_eq!(entries.len(), 3, "Ring buffer should have pruned to max 3");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
