import { useCallback, useEffect, useRef, useState } from "react";
import { useRecommendationsStore } from "../stores/recommendationsStore";
import type { Recommendation, RiskLevel } from "../types/recommendation";

const RISK_COLORS: Record<RiskLevel, string> = {
  critical: "bg-red-600 text-white",
  high: "bg-orange-500 text-white",
  medium: "bg-yellow-500 text-black",
  low: "bg-green-600 text-white",
};

interface ApprovalDialogProps {
  recommendation: Recommendation;
  onClose: () => void;
}

type DialogStep = "review" | "double_confirm" | "reject_reason";

export function ApprovalDialog({ recommendation, onClose }: ApprovalDialogProps) {
  const { approveRecommendation, rejectRecommendation } = useRecommendationsStore();
  const [step, setStep] = useState<DialogStep>("review");
  const [rejectReason, setRejectReason] = useState("");
  const [processing, setProcessing] = useState(false);
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (dialog && !dialog.open) {
      dialog.showModal();
    }
    return () => {
      if (dialog?.open) {
        dialog.close();
      }
    };
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [onClose],
  );

  const handleApprove = async () => {
    if (recommendation.risk_level === "critical") return;

    if (recommendation.risk_level === "high" && step !== "double_confirm") {
      setStep("double_confirm");
      return;
    }

    setProcessing(true);
    try {
      await approveRecommendation(recommendation.id);
      onClose();
    } finally {
      setProcessing(false);
    }
  };

  const handleReject = async () => {
    if (step !== "reject_reason") {
      setStep("reject_reason");
      return;
    }

    setProcessing(true);
    try {
      await rejectRecommendation(recommendation.id, rejectReason.trim() || undefined);
      onClose();
    } finally {
      setProcessing(false);
    }
  };

  const isCritical = recommendation.risk_level === "critical";

  return (
    <dialog
      ref={dialogRef}
      onKeyDown={handleKeyDown}
      className="fixed inset-0 z-50 m-auto w-full max-w-lg rounded-xl border border-gray-700 bg-gray-900 p-0 shadow-2xl backdrop:bg-black/60"
      aria-modal="true"
      aria-labelledby="approval-dialog-title"
    >
      <div className="p-6 text-gray-100">
        <h2
          id="approval-dialog-title"
          className="text-lg font-bold mb-4"
        >
          Review Recommendation
        </h2>

        <div className="space-y-4">
          <Section label="What will change">
            <p>{recommendation.description}</p>
          </Section>

          <Section label="Why">
            <p>Rule: <code className="text-sm bg-gray-800 px-1 rounded">{recommendation.rule_id}</code></p>
            <p className="text-sm text-gray-400 mt-1">Target: {recommendation.target}</p>
          </Section>

          <Section label="Risk level">
            <span
              className={`inline-block px-2 py-1 rounded text-xs font-bold uppercase ${RISK_COLORS[recommendation.risk_level]}`}
            >
              {recommendation.risk_level}
            </span>
          </Section>

          <Section label="Rollback plan">
            <p>{recommendation.rollback_plan ?? "Restore from pre-action snapshot"}</p>
          </Section>

          {isCritical && (
            <div className="p-3 bg-red-900/40 border border-red-700 rounded-lg">
              <p className="text-red-300 text-sm font-medium">
                Critical risk — this cannot be approved. Manual intervention required.
              </p>
            </div>
          )}

          {step === "double_confirm" && (
            <div className="p-3 bg-orange-900/40 border border-orange-700 rounded-lg">
              <p className="text-orange-300 text-sm font-medium">
                This is a high-risk action. Are you sure you want to proceed?
              </p>
            </div>
          )}

          {step === "reject_reason" && (
            <div>
              <label htmlFor="reject-reason" className="text-sm font-medium text-gray-400">
                Rejection reason (optional)
              </label>
              <textarea
                id="reject-reason"
                value={rejectReason}
                onChange={(e) => setRejectReason(e.target.value)}
                className="mt-1 w-full rounded-md border border-gray-600 bg-gray-800 p-2 text-sm text-gray-100 focus:border-blue-500 focus:outline-none"
                rows={3}
                autoFocus
              />
            </div>
          )}
        </div>

        <div className="mt-6 flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-300 bg-gray-800 border border-gray-600 rounded-lg hover:bg-gray-700"
          >
            Cancel
          </button>

          {step !== "reject_reason" && (
            <button
              onClick={handleReject}
              disabled={processing}
              className="px-4 py-2 text-sm font-medium text-red-300 bg-red-900/30 border border-red-700 rounded-lg hover:bg-red-900/50 disabled:opacity-50"
            >
              Reject
            </button>
          )}

          {step === "reject_reason" && (
            <button
              onClick={handleReject}
              disabled={processing}
              className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 disabled:opacity-50"
            >
              {processing ? "Rejecting..." : "Confirm Rejection"}
            </button>
          )}

          {step !== "reject_reason" && !isCritical && (
            <button
              onClick={handleApprove}
              disabled={processing}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {processing
                ? "Approving..."
                : step === "double_confirm"
                  ? "Yes, Approve"
                  : "Approve"}
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <span className="text-xs font-medium text-gray-500 uppercase tracking-wide">{label}</span>
      <div className="text-sm text-gray-200 mt-1">{children}</div>
    </div>
  );
}
