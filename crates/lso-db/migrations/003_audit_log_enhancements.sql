ALTER TABLE audit_log ADD COLUMN category TEXT NOT NULL DEFAULT '';
ALTER TABLE audit_log ADD COLUMN details TEXT;

CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_log_result ON audit_log(result);
CREATE INDEX IF NOT EXISTS idx_audit_log_category ON audit_log(category);
