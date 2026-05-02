//! Probe result storage.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lso_core::{ProbeResult, SensorError};

/// Trait for probe result persistence.
pub trait ProbeStore: Send + Sync {
    /// Store a probe result.
    fn store(&self, result: &ProbeResult) -> Result<(), SensorError>;

    /// Retrieve the most recent result for a given probe ID.
    fn latest(&self, probe_id: &str) -> Result<Option<ProbeResult>, SensorError>;
}

/// File-based JSON storage backend (one JSON-lines file per probe).
pub struct StorageBackend {
    base_dir: PathBuf,
    write_lock: Mutex<()>,
}

impl StorageBackend {
    /// Create a new storage backend rooted at the given directory.
    pub fn new(base_dir: &Path) -> Result<Self, SensorError> {
        fs::create_dir_all(base_dir).map_err(|e| {
            SensorError::Storage(format!("cannot create storage dir: {e}"))
        })?;
        Ok(Self {
            base_dir: base_dir.to_path_buf(),
            write_lock: Mutex::new(()),
        })
    }

    fn probe_file(&self, probe_id: &str) -> PathBuf {
        let safe_name = probe_id.replace('.', "_");
        self.base_dir.join(format!("{safe_name}.jsonl"))
    }
}

impl ProbeStore for StorageBackend {
    fn store(&self, result: &ProbeResult) -> Result<(), SensorError> {
        use std::io::Write;

        let _lock = self.write_lock.lock().map_err(|e| {
            SensorError::Storage(format!("lock poisoned: {e}"))
        })?;

        let path = self.probe_file(&result.probe_id);
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| SensorError::Storage(format!("cannot open {}: {e}", path.display())))?;

        let json = serde_json::to_string(result)
            .map_err(|e| SensorError::Storage(format!("serialization error: {e}")))?;

        writeln!(file, "{json}")
            .map_err(|e| SensorError::Storage(format!("write error: {e}")))?;

        Ok(())
    }

    fn latest(&self, probe_id: &str) -> Result<Option<ProbeResult>, SensorError> {
        let path = self.probe_file(probe_id);
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| SensorError::Storage(format!("read error: {e}")))?;

        let last_line = content.lines().next_back();
        match last_line {
            Some(line) => {
                let result: ProbeResult = serde_json::from_str(line)
                    .map_err(|e| SensorError::Storage(format!("parse error: {e}")))?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lso_core::MetricValue;
    use std::collections::HashMap;

    fn sample_result() -> ProbeResult {
        let mut metrics = HashMap::new();
        metrics.insert("total_bytes".to_string(), MetricValue::Uint(1_000_000));
        metrics.insert("used_bytes".to_string(), MetricValue::Uint(500_000));
        metrics.insert("available_bytes".to_string(), MetricValue::Uint(500_000));
        metrics.insert("usage_percent".to_string(), MetricValue::Float(50.0));
        metrics.insert("mount_point".to_string(), MetricValue::Text("/".to_string()));
        metrics.insert("fs_type".to_string(), MetricValue::Text("ext4".to_string()));

        ProbeResult {
            probe_id: "disk.usage".to_string(),
            timestamp: Utc::now(),
            metrics: vec![metrics],
        }
    }

    #[test]
    fn store_and_retrieve_result() {
        let dir = tempfile::tempdir().unwrap();
        let store = StorageBackend::new(dir.path()).unwrap();
        let result = sample_result();

        store.store(&result).unwrap();

        let latest = store.latest("disk.usage").unwrap().expect("should have result");
        assert_eq!(latest.probe_id, "disk.usage");
        assert_eq!(latest.metrics.len(), 1);
    }

    #[test]
    fn latest_returns_none_for_unknown_probe() {
        let dir = tempfile::tempdir().unwrap();
        let store = StorageBackend::new(dir.path()).unwrap();

        let result = store.latest("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn latest_returns_most_recent() {
        let dir = tempfile::tempdir().unwrap();
        let store = StorageBackend::new(dir.path()).unwrap();

        let mut r1 = sample_result();
        r1.metrics[0].insert("used_bytes".to_string(), MetricValue::Uint(100));
        store.store(&r1).unwrap();

        let mut r2 = sample_result();
        r2.metrics[0].insert("used_bytes".to_string(), MetricValue::Uint(200));
        store.store(&r2).unwrap();

        let latest = store.latest("disk.usage").unwrap().expect("should have result");
        match latest.metrics[0].get("used_bytes") {
            Some(MetricValue::Uint(v)) => assert_eq!(*v, 200),
            _ => panic!("unexpected metric value"),
        }
    }
}
