//! Cross-platform tests using MockPlatform — run on all OSes.

use lso_core::{MockPlatform, Platform, PlatformProvider};

#[test]
fn mock_platform_simulates_all_oses() {
    for platform in [Platform::MacOS, Platform::Linux, Platform::Windows] {
        let mock = MockPlatform::new(platform);
        assert_eq!(mock.platform(), platform);
        assert!(!mock.home_dir().as_os_str().is_empty());
        assert!(!mock.temp_dir().as_os_str().is_empty());
        assert!(!mock.data_dir().as_os_str().is_empty());
        assert!(!mock.config_dir().as_os_str().is_empty());
    }
}

#[test]
fn mock_dirs_are_distinct() {
    let mock = MockPlatform::new(Platform::MacOS);
    assert_ne!(mock.home_dir(), mock.temp_dir());
    assert_ne!(mock.data_dir(), mock.config_dir());
    assert_ne!(mock.home_dir(), mock.data_dir());
}

#[test]
fn mock_filesystem_isolation() {
    let root = std::env::temp_dir().join("lso_test_isolation");
    let mock = MockPlatform::with_root(Platform::Linux, root.clone());
    mock.create_dirs().unwrap();

    let test_file = mock.home_dir().join("test.txt");
    std::fs::write(&test_file, "hello").unwrap();
    assert!(test_file.exists());

    mock.cleanup();
    assert!(!test_file.exists());
    let _ = std::fs::remove_dir_all(&root);
}
