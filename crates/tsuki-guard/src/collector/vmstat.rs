use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use tsuki_common::telemetry::VmstatRecord;

/// Reader for Linux virtual memory statistics from `/proc/vmstat`.
#[derive(Debug, Clone)]
pub struct VmstatReader {
    vmstat_path: PathBuf,
    prev_record: Option<(u64, VmstatRecord)>, // (timestamp_ms, record)
}

impl Default for VmstatReader {
    fn default() -> Self {
        Self {
            vmstat_path: PathBuf::from("/proc/vmstat"),
            prev_record: None,
        }
    }
}

impl VmstatReader {
    pub fn new(vmstat_path: PathBuf) -> Self {
        Self {
            vmstat_path,
            prev_record: None,
        }
    }

    /// Read raw cumulative vmstat counters from file.
    pub fn read_raw(&self) -> io::Result<VmstatRecord> {
        let mut file = File::open(&self.vmstat_path)?;
        let mut buffer = [0u8; 8192];
        let bytes_read = file.read(&mut buffer)?;
        let content = std::str::from_utf8(&buffer[..bytes_read])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(parse_vmstat_content(content))
    }

    /// Read raw counters and compute per-second rates relative to previous sample.
    /// Returns (raw_record, rates_record). If first sample, rates equal 0.
    pub fn read_sample(&mut self, timestamp_ms: u64) -> io::Result<(VmstatRecord, VmstatRecord)> {
        let current = self.read_raw()?;

        let rates = if let Some((prev_ts, ref prev)) = self.prev_record {
            let delta_ms = timestamp_ms.saturating_sub(prev_ts);
            if delta_ms > 0 {
                let delta_sec = delta_ms as f64 / 1000.0;
                compute_rates(prev, &current, delta_sec)
            } else {
                VmstatRecord::default()
            }
        } else {
            VmstatRecord::default()
        };

        self.prev_record = Some((timestamp_ms, current.clone()));
        Ok((current, rates))
    }
}

/// Computes per-second event rates handling counter resets/overflows gracefully.
pub fn compute_rates(prev: &VmstatRecord, curr: &VmstatRecord, delta_sec: f64) -> VmstatRecord {
    if delta_sec <= 0.0 {
        return VmstatRecord::default();
    }

    let diff = |p: u64, c: u64| -> u64 {
        if c >= p {
            ((c - p) as f64 / delta_sec).round() as u64
        } else {
            // Counter reset or overflow
            0
        }
    };

    VmstatRecord {
        pgpgin: diff(prev.pgpgin, curr.pgpgin),
        pgpgout: diff(prev.pgpgout, curr.pgpgout),
        pswpin: diff(prev.pswpin, curr.pswpin),
        pswpout: diff(prev.pswpout, curr.pswpout),
        pgfault: diff(prev.pgfault, curr.pgfault),
        pgmajfault: diff(prev.pgmajfault, curr.pgmajfault),
        workingset_refault: diff(prev.workingset_refault, curr.workingset_refault),
        pgscan_kswapd: diff(prev.pgscan_kswapd, curr.pgscan_kswapd),
        pgscan_direct: diff(prev.pgscan_direct, curr.pgscan_direct),
        pgsteal_kswapd: diff(prev.pgsteal_kswapd, curr.pgsteal_kswapd),
        pgsteal_direct: diff(prev.pgsteal_direct, curr.pgsteal_direct),
        compact_stall: diff(prev.compact_stall, curr.compact_stall),
        oom_kill: diff(prev.oom_kill, curr.oom_kill),
    }
}

/// Parse `/proc/vmstat` key-value pairs without heap allocations.
pub fn parse_vmstat_content(content: &str) -> VmstatRecord {
    let mut record = VmstatRecord::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((key, val_str)) = line.split_once(' ') {
            let val = val_str.trim().parse::<u64>().unwrap_or(0);
            match key {
                "pgpgin" => record.pgpgin = val,
                "pgpgout" => record.pgpgout = val,
                "pswpin" => record.pswpin = val,
                "pswpout" => record.pswpout = val,
                "pgfault" => record.pgfault = val,
                "pgmajfault" => record.pgmajfault = val,
                // On some kernels it's workingset_refault_file / workingset_refault_anon
                k if k.starts_with("workingset_refault") => {
                    record.workingset_refault = record.workingset_refault.saturating_add(val);
                }
                "pgscan_kswapd" => record.pgscan_kswapd = val,
                "pgscan_direct" => record.pgscan_direct = val,
                "pgsteal_kswapd" => record.pgsteal_kswapd = val,
                "pgsteal_direct" => record.pgsteal_direct = val,
                "compact_stall" => record.compact_stall = val,
                "oom_kill" => record.oom_kill = val,
                _ => {}
            }
        }
    }

    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_vmstat() {
        let sample = "nr_free_pages 1000\n\
                      pgpgin 20000\n\
                      pgpgout 50000\n\
                      pswpin 10\n\
                      pswpout 5\n\
                      pgfault 800000\n\
                      pgmajfault 150\n\
                      workingset_refault_file 300\n\
                      workingset_refault_anon 200\n\
                      pgscan_kswapd 1200\n\
                      pgscan_direct 300\n\
                      pgsteal_kswapd 1150\n\
                      pgsteal_direct 290\n\
                      compact_stall 4\n\
                      oom_kill 1\n";

        let rec = parse_vmstat_content(sample);
        assert_eq!(rec.pgpgin, 20000);
        assert_eq!(rec.pgpgout, 50000);
        assert_eq!(rec.pswpin, 10);
        assert_eq!(rec.pswpout, 5);
        assert_eq!(rec.pgfault, 800000);
        assert_eq!(rec.pgmajfault, 150);
        assert_eq!(rec.workingset_refault, 500); // 300 + 200 combined
        assert_eq!(rec.pgscan_kswapd, 1200);
        assert_eq!(rec.pgsteal_direct, 290);
        assert_eq!(rec.compact_stall, 4);
        assert_eq!(rec.oom_kill, 1);
    }

    #[test]
    fn test_rate_computation() {
        let prev = VmstatRecord {
            pgfault: 1000,
            pgmajfault: 10,
            workingset_refault: 50,
            ..Default::default()
        };
        let curr = VmstatRecord {
            pgfault: 1500,
            pgmajfault: 20,
            workingset_refault: 150,
            ..Default::default()
        };

        let rates = compute_rates(&prev, &curr, 1.0);
        assert_eq!(rates.pgfault, 500);
        assert_eq!(rates.pgmajfault, 10);
        assert_eq!(rates.workingset_refault, 100);
    }

    #[test]
    fn test_live_vmstat() {
        if Path::new("/proc/vmstat").exists() {
            let mut reader = VmstatReader::default();
            let res = reader.read_sample(1000);
            assert!(res.is_ok());
            let (raw, _) = res.unwrap();
            assert!(raw.pgfault > 0);
        }
    }
}
