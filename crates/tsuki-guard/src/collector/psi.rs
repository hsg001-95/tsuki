use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tsuki_common::telemetry::{PressureRecord, PsiMetrics};

/// Reader for Linux Pressure Stall Information (PSI) from `/proc/pressure`.
#[derive(Debug, Clone)]
pub struct PsiReader {
    memory_path: PathBuf,
    cpu_path: PathBuf,
    io_path: PathBuf,
}

impl Default for PsiReader {
    fn default() -> Self {
        Self {
            memory_path: PathBuf::from("/proc/pressure/memory"),
            cpu_path: PathBuf::from("/proc/pressure/cpu"),
            io_path: PathBuf::from("/proc/pressure/io"),
        }
    }
}

impl PsiReader {
    /// Create a new reader with custom paths (primarily for testing with mock files).
    pub fn new(memory_path: PathBuf, cpu_path: PathBuf, io_path: PathBuf) -> Self {
        Self {
            memory_path,
            cpu_path,
            io_path,
        }
    }

    /// Read all PSI metrics (memory, cpu, io) into a `PressureRecord`.
    pub fn read_pressure(&self) -> io::Result<PressureRecord> {
        let memory = self.read_file(&self.memory_path)?;
        let cpu = self.read_file(&self.cpu_path)?;
        let io = self.read_file(&self.io_path)?;

        Ok(PressureRecord { cpu, memory, io })
    }

    fn read_file(&self, path: &Path) -> io::Result<PsiMetrics> {
        let mut file = File::open(path)?;
        let mut buffer = [0u8; 512];
        let bytes_read = file.read(&mut buffer)?;
        let content = std::str::from_utf8(&buffer[..bytes_read])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(parse_psi_content(content))
    }
}

/// Parse PSI file content without heap allocations.
/// Expected format:
/// some avg10=0.00 avg60=0.00 avg300=0.00 total=967
/// full avg10=0.00 avg60=0.00 avg300=0.00 total=879
pub fn parse_psi_content(content: &str) -> PsiMetrics {
    let mut metrics = PsiMetrics::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_some = line.starts_with("some");
        let is_full = line.starts_with("full");

        if !is_some && !is_full {
            continue;
        }

        let mut avg10 = 0.0;
        let mut avg60 = 0.0;
        let mut avg300 = 0.0;
        let mut total = 0u64;

        for part in line.split_whitespace().skip(1) {
            if let Some((k, v)) = part.split_once('=') {
                match k {
                    "avg10" => avg10 = v.parse::<f64>().unwrap_or(0.0),
                    "avg60" => avg60 = v.parse::<f64>().unwrap_or(0.0),
                    "avg300" => avg300 = v.parse::<f64>().unwrap_or(0.0),
                    "total" => total = v.parse::<u64>().unwrap_or(0),
                    _ => {}
                }
            }
        }

        if is_some {
            metrics.some_avg10 = avg10;
            metrics.some_avg60 = avg60;
            metrics.some_avg300 = avg300;
            metrics.some_total_us = total;
        } else if is_full {
            metrics.full_avg10 = avg10;
            metrics.full_avg60 = avg60;
            metrics.full_avg300 = avg300;
            metrics.full_total_us = total;
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_psi_memory() {
        let sample = "some avg10=1.25 avg60=0.45 avg300=0.10 total=123456\n\
                      full avg10=0.50 avg60=0.20 avg300=0.05 total=65432\n";

        let metrics = parse_psi_content(sample);
        assert_eq!(metrics.some_avg10, 1.25);
        assert_eq!(metrics.some_avg60, 0.45);
        assert_eq!(metrics.some_avg300, 0.10);
        assert_eq!(metrics.some_total_us, 123456);

        assert_eq!(metrics.full_avg10, 0.50);
        assert_eq!(metrics.full_avg60, 0.20);
        assert_eq!(metrics.full_avg300, 0.05);
        assert_eq!(metrics.full_total_us, 65432);
    }

    #[test]
    fn test_parse_psi_cpu_no_full() {
        let sample = "some avg10=15.30 avg60=5.20 avg300=1.00 total=998877\n";
        let metrics = parse_psi_content(sample);
        assert_eq!(metrics.some_avg10, 15.30);
        assert_eq!(metrics.some_total_us, 998877);
        assert_eq!(metrics.full_avg10, 0.0);
        assert_eq!(metrics.full_total_us, 0);
    }

    #[test]
    fn test_live_proc_pressure() {
        if Path::new("/proc/pressure/memory").exists() {
            let reader = PsiReader::default();
            let res = reader.read_pressure();
            assert!(res.is_ok(), "Failed to read live PSI: {:?}", res.err());
            let p = res.unwrap();
            // Totals are non-negative
            assert!(
                p.cpu.some_total_us > 0 || p.memory.some_total_us > 0 || p.io.some_total_us > 0
            );
        }
    }
}
