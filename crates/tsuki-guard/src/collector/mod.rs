pub mod cgroup;
pub mod psi;
pub mod vmstat;

pub use cgroup::CgroupReader;
pub use psi::PsiReader;
pub use vmstat::VmstatReader;

use std::io;
use std::time::{SystemTime, UNIX_EPOCH};
use tsuki_common::telemetry::TelemetrySample;

/// Main telemetry collector running inside the guard plane.
#[derive(Debug, Default)]
pub struct Collector {
    psi_reader: PsiReader,
    vmstat_reader: VmstatReader,
    cgroup_reader: CgroupReader,
}

impl Collector {
    pub fn new(psi: PsiReader, vmstat: VmstatReader, cgroup: CgroupReader) -> Self {
        Self {
            psi_reader: psi,
            vmstat_reader: vmstat,
            cgroup_reader: cgroup,
        }
    }

    /// Collect a single timestamped snapshot of host telemetry.
    pub fn collect_sample(&mut self) -> io::Result<TelemetrySample> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let pressure = self.psi_reader.read_pressure()?;
        let (_raw_vmstat, vmstat_rates) = self.vmstat_reader.read_sample(timestamp_ms)?;
        let cgroups = self.cgroup_reader.read_top_cgroups().unwrap_or_default();

        Ok(TelemetrySample {
            timestamp_ms,
            pressure,
            vmstat: vmstat_rates, // store rates for the time-series window
            cgroups,
        })
    }
}
