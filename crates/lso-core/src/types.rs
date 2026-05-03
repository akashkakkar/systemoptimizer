//! Shared types for the actuator and snapshot subsystems.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a snapshot.
pub type SnapshotId = String;

/// Risk level of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Required privilege level for an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivilegeLevel {
    User,
    Elevated,
}

/// Approval status for an action request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

/// Metadata for a stored snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: SnapshotId,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub size_estimate: Option<u64>,
    pub platform_detail: String,
}

/// Report from preflight checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub passed: bool,
    pub messages: Vec<String>,
}

/// Result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub changes: Vec<String>,
}

/// Result of verifying an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub verified: bool,
    pub message: String,
}

/// Request to execute an action through the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action_id: String,
    pub target: String,
    pub approval: ApprovalStatus,
}

/// Result of a successful pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub action_id: String,
    pub snapshot_id: SnapshotId,
    pub action_result: ActionResult,
    pub verify_result: VerifyResult,
}
