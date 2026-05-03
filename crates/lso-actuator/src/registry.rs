//! Actuator registry for managing action executors.

use std::collections::HashMap;

use lso_core::{ActionExecutor, ActuatorError};

/// Registry of available action executors.
pub struct ActuatorRegistry {
    executors: HashMap<String, Box<dyn ActionExecutor>>,
}

impl ActuatorRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    /// Register an action executor.
    pub fn register(&mut self, executor: Box<dyn ActionExecutor>) {
        self.executors
            .insert(executor.action_id().to_string(), executor);
    }

    /// Look up an executor by action ID.
    pub fn get(&self, action_id: &str) -> Result<&dyn ActionExecutor, ActuatorError> {
        self.executors
            .get(action_id)
            .map(|e| e.as_ref())
            .ok_or_else(|| ActuatorError::ActionNotFound {
                action_id: action_id.to_string(),
            })
    }

    /// List all registered action IDs.
    pub fn list_actions(&self) -> Vec<&str> {
        self.executors.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ActuatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use lso_core::*;

    struct TestExecutor {
        id: String,
    }

    #[async_trait]
    impl ActionExecutor for TestExecutor {
        fn action_id(&self) -> &str {
            &self.id
        }
        fn description(&self) -> &str {
            "test executor"
        }
        fn risk_level(&self) -> RiskLevel {
            RiskLevel::Low
        }
        fn required_privilege(&self) -> PrivilegeLevel {
            PrivilegeLevel::User
        }
        async fn preflight(&self, _target: &str) -> Result<PreflightReport, ActuatorError> {
            unimplemented!()
        }
        async fn execute(&self, _target: &str) -> Result<ActionResult, ActuatorError> {
            unimplemented!()
        }
        async fn verify(&self, _target: &str) -> Result<VerifyResult, ActuatorError> {
            unimplemented!()
        }
    }

    #[test]
    fn register_and_retrieve() {
        let mut registry = ActuatorRegistry::new();
        registry.register(Box::new(TestExecutor {
            id: "test-1".to_string(),
        }));

        let executor = registry.get("test-1").unwrap();
        assert_eq!(executor.action_id(), "test-1");
    }

    #[test]
    fn get_nonexistent_returns_error() {
        let registry = ActuatorRegistry::new();
        assert!(matches!(
            registry.get("nonexistent"),
            Err(ActuatorError::ActionNotFound { .. })
        ));
    }

    #[test]
    fn list_registered_actions() {
        let mut registry = ActuatorRegistry::new();
        registry.register(Box::new(TestExecutor {
            id: "a".to_string(),
        }));
        registry.register(Box::new(TestExecutor {
            id: "b".to_string(),
        }));

        let mut actions = registry.list_actions();
        actions.sort();
        assert_eq!(actions, vec!["a", "b"]);
    }
}
