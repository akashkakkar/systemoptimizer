//! Application state managed by Tauri.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lso_db::Database;
use lso_engine::{RecommendationManager, RuleEngine};

/// Shared application state accessible from Tauri commands.
pub struct AppState {
    pub recommendation_manager: Mutex<RecommendationManager>,
    pub db: Arc<Database>,
}

impl AppState {
    pub fn init() -> Self {
        let db_path = Self::db_path();
        let db = Arc::new(
            Database::open(&db_path, "lso-default-key").expect("failed to open database"),
        );
        db.migrate().expect("failed to migrate database");

        let rules_dir = Self::rules_dir();
        let engine = if rules_dir.exists() {
            RuleEngine::load_rules(&rules_dir).unwrap_or_else(|_| {
                RuleEngine::from_rules(
                    vec![],
                    lso_core::Platform::detect().unwrap_or(lso_core::Platform::MacOS),
                )
            })
        } else {
            RuleEngine::from_rules(
                vec![],
                lso_core::Platform::detect().unwrap_or(lso_core::Platform::MacOS),
            )
        };

        let rec_db = Database::open(&db_path, "lso-default-key")
            .expect("failed to open recommendation database");
        rec_db.migrate().expect("failed to migrate recommendation database");

        Self {
            recommendation_manager: Mutex::new(RecommendationManager::new(engine, rec_db)),
            db,
        }
    }

    fn db_path() -> PathBuf {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lso");
        std::fs::create_dir_all(&data_dir).ok();
        data_dir.join("lso.db")
    }

    fn rules_dir() -> PathBuf {
        PathBuf::from("rules")
    }
}
