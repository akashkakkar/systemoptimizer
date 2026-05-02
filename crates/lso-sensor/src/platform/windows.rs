//! Windows disk usage — stubbed, returns PlatformNotSupported.

use super::MountStats;
use lso_core::SensorError;

pub fn collect() -> Result<Vec<MountStats>, SensorError> {
    Err(SensorError::PlatformNotSupported(
        "Windows disk probe not yet implemented".to_string(),
    ))
}
