//! Execution pipeline: approval → preflight → snapshot → execute → verify → audit.

use std::sync::Arc;

use chrono::Utc;
use tracing::{info, warn};

use lso_core::{
    ActionRequest, ActuatorError, ApprovalStatus, PipelineResult, SnapshotProvider,
};

use crate::registry::ActuatorRegistry;

/// Orchestrates the approval → preflight → snapshot → execute → verify pipeline.
pub struct ExecutionPipeline {
    registry: ActuatorRegistry,
    snapshot_provider: Arc<dyn SnapshotProvider>,
}

impl ExecutionPipeline {
    /// Create a new pipeline with the given registry and snapshot provider.
    pub fn new(
        registry: ActuatorRegistry,
        snapshot_provider: Arc<dyn SnapshotProvider>,
    ) -> Self {
        Self {
            registry,
            snapshot_provider,
        }
    }

    /// Execute an action through the full safety pipeline.
    pub async fn run(&self, request: &ActionRequest) -> Result<PipelineResult, ActuatorError> {
        // Step 1: Check approval
        if request.approval != ApprovalStatus::Approved {
            return Err(ActuatorError::NotApproved(request.action_id.clone()));
        }

        // Step 2: Look up executor and run preflight
        let executor = self.registry.get(&request.action_id)?;
        let preflight = executor.preflight(&request.target).await?;
        if !preflight.passed {
            return Err(ActuatorError::PreflightFailed(
                preflight.messages.join("; "),
            ));
        }

        // Step 3: Create snapshot
        let label = format!(
            "pre_{}_{}",
            request.action_id,
            Utc::now().format("%Y%m%dT%H%M%S")
        );
        let snapshot_id = self.snapshot_provider.create(&label).await?;

        // Step 4: Execute
        let action_result = match executor.execute(&request.target).await {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    action_id = %request.action_id,
                    snapshot_id = %snapshot_id,
                    error = %e,
                    "execution failed, rolling back"
                );
                self.snapshot_provider
                    .restore(&snapshot_id)
                    .await
                    .map_err(|re| {
                        ActuatorError::RollbackFailed(format!(
                            "original: {e}; rollback: {re}"
                        ))
                    })?;
                return Err(e);
            }
        };

        // Step 5: Verify
        let verify_result = executor.verify(&request.target).await?;

        // Step 6: Audit log
        info!(
            action_id = %request.action_id,
            target = %request.target,
            snapshot_id = %snapshot_id,
            success = action_result.success,
            verified = verify_result.verified,
            "action pipeline completed"
        );

        // Step 7: Auto-rollback on verification failure
        if !verify_result.verified {
            warn!(
                action_id = %request.action_id,
                snapshot_id = %snapshot_id,
                "verification failed, rolling back"
            );
            self.snapshot_provider.restore(&snapshot_id).await?;
            return Err(ActuatorError::VerificationFailed(verify_result.message));
        }

        Ok(PipelineResult {
            action_id: request.action_id.clone(),
            snapshot_id,
            action_result,
            verify_result,
        })
    }

    /// Get a reference to the registry.
    pub fn registry(&self) -> &ActuatorRegistry {
        &self.registry
    }

    /// Get a reference to the snapshot provider.
    pub fn snapshot_provider(&self) -> &Arc<dyn SnapshotProvider> {
        &self.snapshot_provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use lso_core::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockSnapshotProvider {
        restore_called: AtomicBool,
    }

    impl MockSnapshotProvider {
        fn new() -> Self {
            Self {
                restore_called: AtomicBool::new(false),
            }
        }

        fn was_restore_called(&self) -> bool {
            self.restore_called.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl SnapshotProvider for MockSnapshotProvider {
        async fn create(&self, _label: &str) -> Result<SnapshotId, ActuatorError> {
            Ok("mock-snapshot-id".to_string())
        }

        async fn list(&self) -> Result<Vec<SnapshotInfo>, ActuatorError> {
            Ok(vec![])
        }

        async fn restore(&self, _id: &SnapshotId) -> Result<(), ActuatorError> {
            self.restore_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn delete(&self, _id: &SnapshotId) -> Result<(), ActuatorError> {
            Ok(())
        }

        fn supports_platform(&self) -> bool {
            true
        }
    }

    struct MockExecutor {
        fail_execute: bool,
        fail_verify: bool,
    }

    impl MockExecutor {
        fn passing() -> Self {
            Self {
                fail_execute: false,
                fail_verify: false,
            }
        }

        fn failing_verify() -> Self {
            Self {
                fail_execute: false,
                fail_verify: true,
            }
        }

        fn failing_execute() -> Self {
            Self {
                fail_execute: true,
                fail_verify: false,
            }
        }
    }

    #[async_trait]
    impl ActionExecutor for MockExecutor {
        fn action_id(&self) -> &str {
            "mock-action"
        }

        fn description(&self) -> &str {
            "A mock action for testing"
        }

        fn risk_level(&self) -> RiskLevel {
            RiskLevel::Low
        }

        fn required_privilege(&self) -> PrivilegeLevel {
            PrivilegeLevel::User
        }

        async fn preflight(&self, _target: &str) -> Result<PreflightReport, ActuatorError> {
            Ok(PreflightReport {
                passed: true,
                messages: vec!["all checks passed".to_string()],
            })
        }

        async fn execute(&self, _target: &str) -> Result<ActionResult, ActuatorError> {
            if self.fail_execute {
                return Err(ActuatorError::ExecutionFailed("mock failure".to_string()));
            }
            Ok(ActionResult {
                success: true,
                message: "action completed".to_string(),
                changes: vec!["changed something".to_string()],
            })
        }

        async fn verify(&self, _target: &str) -> Result<VerifyResult, ActuatorError> {
            Ok(VerifyResult {
                verified: !self.fail_verify,
                message: if self.fail_verify {
                    "verification failed".to_string()
                } else {
                    "verified ok".to_string()
                },
            })
        }
    }

    fn build_pipeline(
        executor: MockExecutor,
        snapshot: Arc<MockSnapshotProvider>,
    ) -> ExecutionPipeline {
        let mut registry = ActuatorRegistry::new();
        registry.register(Box::new(executor));
        ExecutionPipeline::new(registry, snapshot)
    }

    #[tokio::test]
    async fn rejects_unapproved_action() {
        let snapshot = Arc::new(MockSnapshotProvider::new());
        let pipeline = build_pipeline(MockExecutor::passing(), snapshot);

        let request = ActionRequest {
            action_id: "mock-action".to_string(),
            target: "/tmp/test".to_string(),
            approval: ApprovalStatus::Pending,
        };

        let result = pipeline.run(&request).await;
        assert!(matches!(result, Err(ActuatorError::NotApproved(_))));
    }

    #[tokio::test]
    async fn rejects_denied_action() {
        let snapshot = Arc::new(MockSnapshotProvider::new());
        let pipeline = build_pipeline(MockExecutor::passing(), snapshot);

        let request = ActionRequest {
            action_id: "mock-action".to_string(),
            target: "/tmp/test".to_string(),
            approval: ApprovalStatus::Denied,
        };

        let result = pipeline.run(&request).await;
        assert!(matches!(result, Err(ActuatorError::NotApproved(_))));
    }

    #[tokio::test]
    async fn happy_path_execution() {
        let snapshot = Arc::new(MockSnapshotProvider::new());
        let pipeline = build_pipeline(MockExecutor::passing(), snapshot.clone());

        let request = ActionRequest {
            action_id: "mock-action".to_string(),
            target: "/tmp/test".to_string(),
            approval: ApprovalStatus::Approved,
        };

        let result = pipeline.run(&request).await.unwrap();
        assert_eq!(result.action_id, "mock-action");
        assert!(result.action_result.success);
        assert!(result.verify_result.verified);
        assert!(!snapshot.was_restore_called());
    }

    #[tokio::test]
    async fn failed_verification_triggers_rollback() {
        let snapshot = Arc::new(MockSnapshotProvider::new());
        let pipeline = build_pipeline(MockExecutor::failing_verify(), snapshot.clone());

        let request = ActionRequest {
            action_id: "mock-action".to_string(),
            target: "/tmp/test".to_string(),
            approval: ApprovalStatus::Approved,
        };

        let result = pipeline.run(&request).await;
        assert!(matches!(result, Err(ActuatorError::VerificationFailed(_))));
        assert!(snapshot.was_restore_called());
    }

    #[tokio::test]
    async fn failed_execution_triggers_rollback() {
        let snapshot = Arc::new(MockSnapshotProvider::new());
        let pipeline = build_pipeline(MockExecutor::failing_execute(), snapshot.clone());

        let request = ActionRequest {
            action_id: "mock-action".to_string(),
            target: "/tmp/test".to_string(),
            approval: ApprovalStatus::Approved,
        };

        let result = pipeline.run(&request).await;
        assert!(matches!(result, Err(ActuatorError::ExecutionFailed(_))));
        assert!(snapshot.was_restore_called());
    }

    #[tokio::test]
    async fn action_not_found_returns_error() {
        let snapshot = Arc::new(MockSnapshotProvider::new());
        let pipeline = build_pipeline(MockExecutor::passing(), snapshot);

        let request = ActionRequest {
            action_id: "nonexistent-action".to_string(),
            target: "/tmp/test".to_string(),
            approval: ApprovalStatus::Approved,
        };

        let result = pipeline.run(&request).await;
        assert!(matches!(result, Err(ActuatorError::ActionNotFound { .. })));
    }
}
