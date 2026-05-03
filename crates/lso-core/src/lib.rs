//! LSO Core — shared types, traits, and error definitions.

pub mod error;
pub mod traits;
pub mod types;

pub use error::ActuatorError;
pub use traits::{ActionExecutor, SnapshotProvider};
pub use types::{
    ActionRequest, ActionResult, ApprovalStatus, PipelineResult, PreflightReport, PrivilegeLevel,
    RiskLevel, SnapshotId, SnapshotInfo, VerifyResult,
};
