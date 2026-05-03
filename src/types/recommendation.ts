export type RiskLevel = "low" | "medium" | "high" | "critical";

export type RecommendationStatus =
  | "pending"
  | "approved"
  | "rejected"
  | "expired"
  | "executing"
  | "succeeded"
  | "failed"
  | "rolled_back";

export interface Recommendation {
  id: string;
  rule_id: string;
  title: string;
  description: string;
  risk_level: RiskLevel;
  category: string;
  target: string;
  rollback_plan: string | null;
  status: RecommendationStatus;
  rejection_reason: string | null;
  created_at: string;
  resolved_at: string | null;
}

export interface ApprovalResponse {
  id: string;
  status: RecommendationStatus;
  requires_double_confirm: boolean;
}
