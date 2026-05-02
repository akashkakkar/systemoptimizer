//! LSO Engine — rule engine and recommendation generator.

pub mod evaluator;
pub mod rule;

pub use evaluator::RuleEngine;
pub use rule::{Condition, Operator, Rule, RuleMetadata, RuleRecommendation};

use thiserror::Error;

/// Errors that can occur in the engine crate.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("failed to read rules directory '{path}': {source}")]
    ReadDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to read rule file '{path}': {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },

    #[error("{path}: {message}")]
    Validation { path: String, message: String },

    #[error("{path}: TOML parse error: {source}")]
    TomlParse {
        path: String,
        source: toml::de::Error,
    },
}
