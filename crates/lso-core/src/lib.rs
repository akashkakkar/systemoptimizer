//! LSO Core — shared types, traits, and error definitions.

mod error;
mod platform;
mod probe;

pub use error::SensorError;
pub use platform::Platform;
pub use probe::{MetricValue, PrivilegeLevel, ProbeResult, SystemProbe};
