ALTER TABLE recommendations ADD COLUMN rule_id TEXT NOT NULL DEFAULT '';
ALTER TABLE recommendations ADD COLUMN target TEXT NOT NULL DEFAULT '';
ALTER TABLE recommendations ADD COLUMN rejection_reason TEXT;
ALTER TABLE recommendations ADD COLUMN rollback_plan TEXT;

CREATE INDEX IF NOT EXISTS idx_recommendations_status ON recommendations(status);
CREATE INDEX IF NOT EXISTS idx_recommendations_rule_target ON recommendations(rule_id, target);
