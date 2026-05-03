import type { RiskLevel } from "./recommendation";

export type ActionResult = "success" | "failed" | "rolled_back";
export type CleanupTarget = "system_temp" | "user_cache" | "app_logs";
export type ExportFormat = "json" | "csv";

export interface AuditEntry {
  id: string;
  timestamp: string;
  action: string;
  target: string;
  category: string;
  risk_level: RiskLevel;
  user_approved: boolean;
  snapshot_id: string | null;
  result: ActionResult;
  details: string | null;
  rollback_available: boolean;
}

export interface AuditFilter {
  status?: ActionResult;
  risk_level?: RiskLevel;
  category?: string;
  date_from?: string;
  date_to?: string;
  search?: string;
  page?: number;
  per_page?: number;
}

export interface AuditPage {
  entries: AuditEntry[];
  total_count: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface SkippedFile {
  path: string;
  reason: string;
}

export interface PreflightReport {
  target: CleanupTarget;
  file_count: number;
  total_bytes: number;
  oldest_modified: string | null;
  newest_modified: string | null;
  skipped: SkippedFile[];
}

export interface CleanupResult {
  files_deleted: number;
  bytes_reclaimed: number;
  errors: string[];
  skipped: SkippedFile[];
  snapshot_id: string;
  duration_ms: number;
}
