//! StartupDisableExecutor — disables a startup item with snapshot + rollback.
//!
//! Risk level: Medium (snapshot + single confirm).
//! Linux: `systemctl disable` for services, rename for .desktop files.
//! macOS / Windows: stubbed.

use lso_core::{ActuatorError, RiskLevel, StartupType};
use serde::{Deserialize, Serialize};

/// Request to disable a startup item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisableRequest {
    pub item_name: String,
    pub item_type: StartupType,
    pub platform_id: String,
}

/// Result of a disable/rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisableResult {
    Disabled { description: String },
    ReEnabled { description: String },
    Failed { error: String },
}

/// Executor for disabling startup items.
///
/// Uses the actuator's `SnapshotManager` for desktop-file rollback.
/// Systemd services are re-enabled via `systemctl enable` on rollback.
pub struct StartupDisableExecutor;

impl StartupDisableExecutor {
    pub fn action_id(&self) -> &str {
        "startup.disable"
    }

    pub fn risk_level(&self) -> RiskLevel {
        RiskLevel::Medium
    }

    /// Validate that the operation can proceed.
    pub fn preflight(&self, request: &DisableRequest) -> Result<(), ActuatorError> {
        match request.item_type {
            StartupType::SystemdService => {
                #[cfg(target_os = "linux")]
                {
                    verify_systemd_unit_exists(&request.platform_id)?;
                }
                #[cfg(not(target_os = "linux"))]
                {
                    return Err(ActuatorError::ExecutionFailed {
                        action: "preflight".into(),
                        reason: "systemd requires Linux".into(),
                    });
                }
            }
            StartupType::DesktopAutostart => {
                #[cfg(unix)]
                {
                    verify_desktop_file_exists(&request.platform_id)?;
                }
                #[cfg(not(unix))]
                {
                    return Err(ActuatorError::ExecutionFailed {
                        action: "preflight".into(),
                        reason: "XDG autostart requires Unix".into(),
                    });
                }
            }
            other => {
                return Err(ActuatorError::ExecutionFailed {
                    action: "preflight".into(),
                    reason: format!("disable not implemented for {other}"),
                });
            }
        }
        Ok(())
    }

    /// Execute the disable. Only call after user approval.
    pub fn execute(&self, request: &DisableRequest) -> Result<DisableResult, ActuatorError> {
        match request.item_type {
            StartupType::SystemdService => {
                #[cfg(target_os = "linux")]
                {
                    disable_systemd(&request.platform_id)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(ActuatorError::ExecutionFailed {
                        action: "disable".into(),
                        reason: "systemd requires Linux".into(),
                    })
                }
            }
            StartupType::DesktopAutostart => {
                #[cfg(unix)]
                {
                    disable_desktop_autostart(&request.platform_id)
                }
                #[cfg(not(unix))]
                {
                    Err(ActuatorError::ExecutionFailed {
                        action: "disable".into(),
                        reason: "XDG autostart requires Unix".into(),
                    })
                }
            }
            other => Err(ActuatorError::ExecutionFailed {
                action: "disable".into(),
                reason: format!("not implemented for {other}"),
            }),
        }
    }

    /// Rollback: re-enable the item.
    pub fn rollback(&self, request: &DisableRequest) -> Result<DisableResult, ActuatorError> {
        match request.item_type {
            StartupType::SystemdService => {
                #[cfg(target_os = "linux")]
                {
                    enable_systemd(&request.platform_id)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    Err(ActuatorError::RollbackFailed(
                        "systemd requires Linux".into(),
                    ))
                }
            }
            StartupType::DesktopAutostart => {
                #[cfg(unix)]
                {
                    enable_desktop_autostart(&request.platform_id)
                }
                #[cfg(not(unix))]
                {
                    Err(ActuatorError::RollbackFailed(
                        "XDG autostart requires Unix".into(),
                    ))
                }
            }
            other => Err(ActuatorError::RollbackFailed(format!(
                "rollback not implemented for {other}"
            ))),
        }
    }
}

// --- Linux systemd operations ---

#[cfg(target_os = "linux")]
fn verify_systemd_unit_exists(unit: &str) -> Result<(), ActuatorError> {
    let output = std::process::Command::new("systemctl")
        .args(["cat", unit])
        .output()
        .map_err(|e| ActuatorError::ExecutionFailed {
            action: "preflight".into(),
            reason: format!("failed to run systemctl: {e}"),
        })?;

    if !output.status.success() {
        return Err(ActuatorError::ExecutionFailed {
            action: "preflight".into(),
            reason: format!("systemd unit '{unit}' not found"),
        });
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn disable_systemd(unit: &str) -> Result<DisableResult, ActuatorError> {
    let output = std::process::Command::new("systemctl")
        .args(["disable", unit])
        .output()
        .map_err(|e| ActuatorError::ExecutionFailed {
            action: "disable".into(),
            reason: e.to_string(),
        })?;

    if output.status.success() {
        Ok(DisableResult::Disabled {
            description: format!("Disabled systemd service '{unit}'"),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(DisableResult::Failed {
            error: format!("systemctl disable failed: {stderr}"),
        })
    }
}

#[cfg(target_os = "linux")]
fn enable_systemd(unit: &str) -> Result<DisableResult, ActuatorError> {
    let output = std::process::Command::new("systemctl")
        .args(["enable", unit])
        .output()
        .map_err(|e| ActuatorError::ExecutionFailed {
            action: "rollback".into(),
            reason: e.to_string(),
        })?;

    if output.status.success() {
        Ok(DisableResult::ReEnabled {
            description: format!("Re-enabled systemd service '{unit}'"),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ActuatorError::RollbackFailed(format!(
            "failed to re-enable '{unit}': {stderr}"
        )))
    }
}

// --- XDG Desktop autostart operations ---

#[cfg(unix)]
fn autostart_path(file_stem: &str) -> Result<std::path::PathBuf, ActuatorError> {
    let home = std::env::var("HOME").map_err(|_| ActuatorError::ExecutionFailed {
        action: "autostart".into(),
        reason: "$HOME not set".into(),
    })?;
    Ok(std::path::PathBuf::from(home)
        .join(".config/autostart")
        .join(format!("{file_stem}.desktop")))
}

#[cfg(unix)]
fn verify_desktop_file_exists(file_stem: &str) -> Result<(), ActuatorError> {
    let path = autostart_path(file_stem)?;
    if !path.exists() {
        return Err(ActuatorError::ExecutionFailed {
            action: "preflight".into(),
            reason: format!("autostart file not found: {}", path.display()),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn disable_desktop_autostart(file_stem: &str) -> Result<DisableResult, ActuatorError> {
    let path = autostart_path(file_stem)?;
    let disabled_path = path.with_extension("desktop.disabled");

    std::fs::rename(&path, &disabled_path).map_err(|e| ActuatorError::ExecutionFailed {
        action: "disable autostart".into(),
        reason: format!("failed to rename {}: {e}", path.display()),
    })?;

    Ok(DisableResult::Disabled {
        description: format!("Disabled autostart item '{file_stem}' (renamed to .disabled)"),
    })
}

#[cfg(unix)]
fn enable_desktop_autostart(file_stem: &str) -> Result<DisableResult, ActuatorError> {
    let path = autostart_path(file_stem)?;
    let disabled_path = path.with_extension("desktop.disabled");

    if !disabled_path.exists() {
        return Err(ActuatorError::RollbackFailed(format!(
            "disabled file not found: {}",
            disabled_path.display()
        )));
    }

    std::fs::rename(&disabled_path, &path).map_err(|e| {
        ActuatorError::RollbackFailed(format!(
            "failed to rename {} back: {e}",
            disabled_path.display()
        ))
    })?;

    Ok(DisableResult::ReEnabled {
        description: format!("Re-enabled autostart item '{file_stem}'"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_id_and_risk() {
        let exec = StartupDisableExecutor;
        assert_eq!(exec.action_id(), "startup.disable");
        assert_eq!(exec.risk_level(), RiskLevel::Medium);
    }

    #[test]
    fn preflight_rejects_unsupported_type() {
        let exec = StartupDisableExecutor;
        let req = DisableRequest {
            item_name: "test".into(),
            item_type: StartupType::LaunchAgent,
            platform_id: "test".into(),
        };
        let result = exec.preflight(&req);
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn desktop_disable_rollback_cycle() {
        let fake_home = std::env::temp_dir().join("lso_test_actuator_home");
        let fake_autostart = fake_home.join(".config/autostart");
        let _ = std::fs::create_dir_all(&fake_autostart);
        std::fs::write(
            fake_autostart.join("testapp.desktop"),
            "[Desktop Entry]\nName=Test\nExec=/bin/true\n",
        )
        .unwrap();

        let original_home = std::env::var("HOME").unwrap();
        std::env::set_var("HOME", &fake_home);

        let exec = StartupDisableExecutor;
        let req = DisableRequest {
            item_name: "testapp".into(),
            item_type: StartupType::DesktopAutostart,
            platform_id: "testapp".into(),
        };

        // Preflight
        assert!(exec.preflight(&req).is_ok());

        // Disable
        let result = exec.execute(&req).unwrap();
        assert!(matches!(result, DisableResult::Disabled { .. }));
        assert!(!fake_autostart.join("testapp.desktop").exists());
        assert!(fake_autostart.join("testapp.desktop.disabled").exists());

        // Rollback
        let result = exec.rollback(&req).unwrap();
        assert!(matches!(result, DisableResult::ReEnabled { .. }));
        assert!(fake_autostart.join("testapp.desktop").exists());

        std::env::set_var("HOME", original_home);
        let _ = std::fs::remove_dir_all(&fake_home);
    }
}
