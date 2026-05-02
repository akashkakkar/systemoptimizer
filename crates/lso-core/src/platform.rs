/// Platform abstraction for OS-specific directory resolution.
use std::path::PathBuf;

use crate::error::LsoError;
use crate::types::Platform;

/// Provides platform-specific directory paths.
pub trait PlatformProvider: Send + Sync {
    fn platform(&self) -> Platform;
    fn home_dir(&self) -> PathBuf;
    fn temp_dir(&self) -> PathBuf;
    fn data_dir(&self) -> PathBuf;
    fn config_dir(&self) -> PathBuf;
}

/// Platform provider backed by the actual OS.
pub struct NativePlatformProvider {
    platform: Platform,
}

impl NativePlatformProvider {
    /// Create a new provider, detecting the current platform.
    pub fn new() -> Result<Self, LsoError> {
        let platform = Platform::detect().ok_or_else(|| {
            LsoError::PlatformNotSupported(std::env::consts::OS.to_string())
        })?;
        Ok(Self { platform })
    }
}

impl PlatformProvider for NativePlatformProvider {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn home_dir(&self) -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
    }

    fn temp_dir(&self) -> PathBuf {
        std::env::temp_dir()
    }

    fn data_dir(&self) -> PathBuf {
        dirs::data_dir()
            .map(|p| p.join("lso"))
            .unwrap_or_else(|| self.home_dir().join(".lso"))
    }

    fn config_dir(&self) -> PathBuf {
        dirs::config_dir()
            .map(|p| p.join("lso"))
            .unwrap_or_else(|| self.home_dir().join(".config").join("lso"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_provider_detects_platform() {
        let provider = NativePlatformProvider::new().expect("should detect platform");
        let platform = provider.platform();
        assert!(
            matches!(platform, Platform::MacOS | Platform::Linux | Platform::Windows),
            "detected platform should be a known variant"
        );
    }

    #[test]
    fn native_provider_returns_nonempty_dirs() {
        let provider = NativePlatformProvider::new().expect("should detect platform");
        assert!(!provider.home_dir().as_os_str().is_empty());
        assert!(!provider.temp_dir().as_os_str().is_empty());
        assert!(!provider.data_dir().as_os_str().is_empty());
        assert!(!provider.config_dir().as_os_str().is_empty());
    }

    #[test]
    fn data_dir_contains_lso() {
        let provider = NativePlatformProvider::new().expect("should detect platform");
        let data = provider.data_dir();
        assert!(
            data.ends_with("lso"),
            "data_dir should end with 'lso', got: {data:?}"
        );
    }

    #[test]
    fn config_dir_contains_lso() {
        let provider = NativePlatformProvider::new().expect("should detect platform");
        let config = provider.config_dir();
        assert!(
            config.ends_with("lso"),
            "config_dir should end with 'lso', got: {config:?}"
        );
    }
}
