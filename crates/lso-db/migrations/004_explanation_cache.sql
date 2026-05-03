-- F16: persistent cache for LLM-generated recommendation explanations.
--
-- Keyed by recommendation id + a sha256 of the inputs that determine
-- the explanation (rule fields + relevant probe metrics). A row is
-- reused only while its data_hash matches; otherwise the explanation
-- is regenerated.
CREATE TABLE IF NOT EXISTS explanation_cache (
    rec_id TEXT PRIMARY KEY,
    data_hash TEXT NOT NULL,
    explanation TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_explanation_cache_hash
    ON explanation_cache(data_hash);
