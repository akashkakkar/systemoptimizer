//! Application state managed by Tauri.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use lso_ai::ollama::OllamaBackend;
use lso_ai::{AiConfig, ExplanationService, LlmProvider};
use lso_core::{ExplanationCache, ProbeResult};
use lso_db::{Database, SqlCipherExplanationCache};
use lso_engine::{RecommendationManager, RuleEngine};

/// Shared application state accessible from Tauri commands.
pub struct AppState {
    pub recommendation_manager: Mutex<RecommendationManager>,
    pub db: Arc<Database>,
    pub explanation_service: Arc<ExplanationService>,
    pub latest_probes: Mutex<Vec<ProbeResult>>,
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

        let explanation_service = Self::init_explanation_service(db.clone());

        Self {
            recommendation_manager: Mutex::new(RecommendationManager::new(engine, rec_db)),
            db,
            explanation_service: Arc::new(explanation_service),
            latest_probes: Mutex::new(Vec::new()),
        }
    }

    fn init_explanation_service(db: Arc<Database>) -> ExplanationService {
        let config = AiConfig::default();
        let llm: Arc<dyn LlmProvider> = Arc::new(
            OllamaBackend::new(config).expect("default AiConfig is always valid"),
        );
        let cache: Arc<dyn ExplanationCache> =
            Arc::new(SqlCipherExplanationCache::new(db));
        ExplanationService::new(llm).with_cache(cache)
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
