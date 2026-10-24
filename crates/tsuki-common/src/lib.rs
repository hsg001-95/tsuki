pub mod config;
pub mod error;
pub mod intent;
pub mod telemetry;

pub use config::Config;
pub use error::{Result, TsukiError};
pub use intent::{ActionState, ActionTemplate, JournalEntry, RemediationIntent};
pub use telemetry::{EvidenceSnapshot, PressureRecord, TelemetrySample, VmstatRecord};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_parses() {
        let cfg = Config::default();
        assert_eq!(cfg.general.mode, "shadow");
        assert_eq!(cfg.guard.sampling_rate_hz, 1.0);
    }
}
