//! Mock platform provider for cross-platform unit tests.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::platform::PlatformProvider;
use crate::types::Platform;

/// Mock platform provider for testing platform-dependent code without
/// running on the actual target OS.
pub struct MockPlatform {
    platform: Platform,
    home: PathBuf,
    temp: PathBuf,
    data: PathBuf,
    config: PathBuf,
    pub mock_data: HashMap<String, Vec<u8>>,
}

impl MockPlatform {
    pub fn new(platform: Platform) -> Self {
        let base = std::env::temp_dir().join("lso_mock");
        Self {
            platform,
            home: base.join("home"),
            temp: base.join("tmp"),
            data: base.join("data"),
            config: base.join("config"),
            mock_data: HashMap::new(),
        }
    }

    /// Create a mock with a custom temp root to avoid test collisions.
    pub fn with_root(platform: Platform, root: PathBuf) -> Self {
        Self {
            platform,
            home: root.join("home"),
            temp: root.join("tmp"),
            data: root.join("data"),
            config: root.join("config"),
            mock_data: HashMap::new(),
        }
    }

    /// Insert mock data that tests can retrieve by key.
    pub fn set_data(&mut self, key: impl Into<String>, data: Vec<u8>) {
        self.mock_data.insert(key.into(), data);
    }

    /// Retrieve mock data by key.
    pub fn get_data(&self, key: &str) -> Option<&[u8]> {
        self.mock_data.get(key).map(|v| v.as_slice())
    }

    /// Ensure all mock directories exist on disk.
    pub fn create_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.home)?;
        std::fs::create_dir_all(&self.temp)?;
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.config)?;
        Ok(())
    }

    /// Remove all mock directories.
    pub fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.home);
        let _ = std::fs::remove_dir_all(&self.temp);
        let _ = std::fs::remove_dir_all(&self.data);
        let _ = std::fs::remove_dir_all(&self.config);
    }
}

impl PlatformProvider for MockPlatform {
    fn platform(&self) -> Platform {
        self.platform
    }

    fn home_dir(&self) -> PathBuf {
        self.home.clone()
    }

    fn temp_dir(&self) -> PathBuf {
        self.temp.clone()
    }

    fn data_dir(&self) -> PathBuf {
        self.data.clone()
    }

    fn config_dir(&self) -> PathBuf {
        self.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provides_platform_dirs() {
        let mock = MockPlatform::new(Platform::MacOS);
        assert_eq!(mock.platform(), Platform::MacOS);
        assert!(mock.home_dir().to_string_lossy().contains("lso_mock"));
        assert!(mock.temp_dir().to_string_lossy().contains("lso_mock"));
        assert!(mock.data_dir().to_string_lossy().contains("lso_mock"));
        assert!(mock.config_dir().to_string_lossy().contains("lso_mock"));
    }

    #[test]
    fn mock_data_roundtrip() {
        let mut mock = MockPlatform::new(Platform::Linux);
        mock.set_data("disk.usage", b"test data".to_vec());
        assert_eq!(mock.get_data("disk.usage"), Some(b"test data".as_slice()));
        assert_eq!(mock.get_data("missing"), None);
    }

    #[test]
    fn mock_with_custom_root() {
        let root = std::env::temp_dir().join("lso_test_custom_root");
        let mock = MockPlatform::with_root(Platform::Windows, root.clone());
        assert_eq!(mock.platform(), Platform::Windows);
        assert_eq!(mock.home_dir(), root.join("home"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn create_and_cleanup_dirs() {
        let root = std::env::temp_dir().join("lso_mock_cleanup_test");
        let mock = MockPlatform::with_root(Platform::Linux, root.clone());
        mock.create_dirs().unwrap();
        assert!(mock.home_dir().exists());
        assert!(mock.temp_dir().exists());
        mock.cleanup();
        assert!(!mock.home_dir().exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mock_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockPlatform>();
    }
}
