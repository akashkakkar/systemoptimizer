//! Windows disk usage — stubbed pending implementation.

use super::MountStats;
use lso_core::SensorError;

pub fn collect() -> Result<Vec<MountStats>, SensorError> {
    Err(SensorError::ProbeFailed {
        probe: "disk.usage".to_string(),
        reason: "Windows disk probe not yet implemented".to_string(),
    })
}
