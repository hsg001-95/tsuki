use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tsuki_common::telemetry::CgroupStatRecord;

/// High-performance reader for cgroup v2 hierarchy statistics.
/// Caches discovered cgroup paths and refreshes periodically to avoid
/// sysfs directory traversal overhead during high-frequency sampling (PERF-02, PERF-03).
#[derive(Debug, Clone)]
pub struct CgroupReader {
    cgroup_root: PathBuf,
    max_cgroups: usize,
    cached_paths: Vec<PathBuf>,
    last_discovery: Option<Instant>,
    discovery_interval: Duration,
}

impl Default for CgroupReader {
    fn default() -> Self {
        Self {
            cgroup_root: PathBuf::from("/sys/fs/cgroup"),
            max_cgroups: 8,
            cached_paths: Vec::new(),
            last_discovery: None,
            discovery_interval: Duration::from_secs(10),
        }
    }
}

impl CgroupReader {
    pub fn new(cgroup_root: PathBuf, max_cgroups: usize) -> Self {
        Self {
            cgroup_root,
            max_cgroups,
            cached_paths: Vec::new(),
            last_discovery: None,
            discovery_interval: Duration::from_secs(10),
        }
    }

    /// Read stats for top cgroups using cached candidate paths.
    pub fn read_top_cgroups(&mut self) -> io::Result<Vec<CgroupStatRecord>> {
        // Refresh cgroup path cache if expired or empty
        let should_refresh = match self.last_discovery {
            Some(last) => last.elapsed() > self.discovery_interval,
            None => true,
        };

        if should_refresh || self.cached_paths.is_empty() {
            self.refresh_candidate_paths();
            self.last_discovery = Some(Instant::now());
        }

        let mut records = Vec::with_capacity(self.cached_paths.len());
        for path in &self.cached_paths {
            if let Ok(rec) = self.read_single_cgroup(path) {
                if rec.memory_current_bytes > 0 {
                    records.push(rec);
                }
            }
        }

        // Sort descending by current memory usage
        records.sort_by_key(|a| std::cmp::Reverse(a.memory_current_bytes));

        if records.len() > self.max_cgroups {
            records.truncate(self.max_cgroups);
        }

        Ok(records)
    }

    /// Discover top-level and second-level cgroup paths.
    fn refresh_candidate_paths(&mut self) {
        let mut paths = Vec::new();

        // 1. Always inspect top slices
        for slice_name in &["user.slice", "system.slice"] {
            let slice_path = self.cgroup_root.join(slice_name);
            if slice_path.exists() {
                paths.push(slice_path.clone());

                // Read direct children (e.g. user-1000.slice or top services)
                if let Ok(entries) = fs::read_dir(&slice_path) {
                    for entry in entries.flatten().take(16) {
                        let p = entry.path();
                        if p.is_dir() && p.join("memory.current").exists() {
                            paths.push(p);
                        }
                    }
                }
            }
        }

        self.cached_paths = paths;
    }

    pub fn read_single_cgroup(&self, cgroup_path: &Path) -> io::Result<CgroupStatRecord> {
        let rel_path = cgroup_path
            .strip_prefix(&self.cgroup_root)
            .unwrap_or(cgroup_path)
            .to_string_lossy()
            .to_string();

        let formatted_path = if rel_path.is_empty() {
            "/".to_string()
        } else if !rel_path.starts_with('/') {
            format!("/{}", rel_path)
        } else {
            rel_path
        };

        let memory_current = read_u64_file(&cgroup_path.join("memory.current")).unwrap_or(0);
        let (cpu_usage_usec, cpu_weight) = read_cpu_stats(cgroup_path);

        Ok(CgroupStatRecord {
            cgroup_path: formatted_path,
            memory_current_bytes: memory_current,
            memory_high_bytes: None,
            memory_max_bytes: None,
            cpu_usage_usec,
            cpu_weight,
            io_bytes: 0,
        })
    }
}

fn read_u64_file(path: &Path) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let mut buf = [0u8; 64];
    let n = file.read(&mut buf)?;
    let s = std::str::from_utf8(&buf[..n])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .trim();
    s.parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_cpu_stats(cgroup_dir: &Path) -> (u64, Option<u64>) {
    let mut usage_usec = 0u64;
    if let Ok(mut file) = File::open(cgroup_dir.join("cpu.stat")) {
        let mut buf = [0u8; 512];
        if let Ok(n) = file.read(&mut buf) {
            if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                for line in s.lines() {
                    if let Some((k, v)) = line.split_once(' ') {
                        if k == "usage_usec" {
                            usage_usec = v.trim().parse::<u64>().unwrap_or(0);
                            break;
                        }
                    }
                }
            }
        }
    }

    let weight = read_u64_file(&cgroup_dir.join("cpu.weight")).ok();
    (usage_usec, weight)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_cgroup_scan() {
        let root = Path::new("/sys/fs/cgroup");
        if root.exists() {
            let mut reader = CgroupReader::new(root.to_path_buf(), 8);
            let cgroups = reader.read_top_cgroups();
            assert!(cgroups.is_ok());
            let list = cgroups.unwrap();
            assert!(!list.is_empty(), "Should discover active cgroups on system");
            if list.len() > 1 {
                assert!(list[0].memory_current_bytes >= list[1].memory_current_bytes);
            }
        }
    }
}
