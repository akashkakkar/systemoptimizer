import { useEffect, useMemo, useState } from "react";
import { useRecommendationsStore } from "../stores/recommendationsStore";
import type {
  Recommendation,
  RecommendationStatus,
  RiskLevel,
} from "../types/recommendation";
import { ApprovalDialog } from "./ApprovalDialog";

const RISK_ORDER: Record<RiskLevel, number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
};

const RISK_COLORS: Record<RiskLevel, string> = {
  critical: "bg-red-600 text-white",
  high: "bg-orange-500 text-white",
  medium: "bg-yellow-500 text-black",
  low: "bg-green-600 text-white",
};

const CATEGORY_LABELS: Record<string, string> = {
  disk: "Disk",
  security: "Security",
  startup: "Startup",
  memory: "Memory",
  network: "Network",
  cleanup: "Cleanup",
};

const STATUS_FILTERS: Array<{ value: RecommendationStatus | "all"; label: string }> = [
  { value: "all", label: "All" },
  { value: "pending", label: "Pending" },
  { value: "approved", label: "Approved" },
  { value: "rejected", label: "Rejected" },
];

export function RecommendationList() {
  const {
    recommendations,
    loading,
    error,
    filter,
    setFilter,
    scanSystem,
    fetchRecommendations,
  } = useRecommendationsStore();

  const [selectedRec, setSelectedRec] = useState<Recommendation | null>(null);

  useEffect(() => {
    fetchRecommendations();
  }, [fetchRecommendations]);

  const filtered = useMemo(() => {
    const recs =
      filter === "all"
        ? recommendations
        : recommendations.filter((r) => r.status === filter);
    return [...recs].sort((a, b) => RISK_ORDER[a.risk_level] - RISK_ORDER[b.risk_level]);
  }, [recommendations, filter]);

  const grouped = useMemo(() => {
    const groups: Record<string, Recommendation[]> = {};
    for (const rec of filtered) {
      const key = rec.category;
      if (!groups[key]) {
        groups[key] = [];
      }
      groups[key].push(rec);
    }
    return groups;
  }, [filtered]);

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold text-gray-100">Recommendations</h2>
        <button
          onClick={scanSystem}
          disabled={loading}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 text-sm font-medium"
        >
          {loading ? "Scanning..." : "Scan System"}
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
          {error}
        </div>
      )}

      <div className="flex gap-2 mb-6" role="tablist">
        {STATUS_FILTERS.map((f) => (
          <button
            key={f.value}
            role="tab"
            aria-selected={filter === f.value}
            onClick={() => setFilter(f.value)}
            className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
              filter === f.value
                ? "bg-blue-600/20 text-blue-400 border border-blue-600/40"
                : "bg-gray-800 text-gray-400 border border-gray-700 hover:bg-gray-750"
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>

      {filtered.length === 0 && !loading && (
        <p className="text-gray-500 text-center py-8">No recommendations found.</p>
      )}

      {Object.entries(grouped).map(([category, recs]) => (
        <div key={category} className="mb-6">
          <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wide mb-3">
            {CATEGORY_LABELS[category] ?? category}
          </h3>
          <div className="space-y-2">
            {recs.map((rec) => (
              <RecommendationCard
                key={rec.id}
                recommendation={rec}
                onSelect={() => setSelectedRec(rec)}
              />
            ))}
          </div>
        </div>
      ))}

      {selectedRec && (
        <ApprovalDialog
          recommendation={selectedRec}
          onClose={() => setSelectedRec(null)}
        />
      )}
    </div>
  );
}

function RecommendationCard({
  recommendation,
  onSelect,
}: {
  recommendation: Recommendation;
  onSelect: () => void;
}) {
  return (
    <div className="border border-gray-700 rounded-lg p-4 bg-gray-800/50 hover:border-gray-600 transition-colors">
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span
              className={`px-2 py-0.5 rounded text-xs font-bold uppercase ${RISK_COLORS[recommendation.risk_level]}`}
            >
              {recommendation.risk_level}
            </span>
            <span className="text-xs text-gray-500 truncate">{recommendation.target}</span>
          </div>
          <h4 className="font-medium text-gray-100 text-sm">{recommendation.title}</h4>
          <p className="text-xs text-gray-400 mt-1 line-clamp-2">{recommendation.description}</p>
        </div>
        {recommendation.status === "pending" && (
          <button
            onClick={onSelect}
            className="ml-3 px-3 py-1.5 bg-blue-600/10 text-blue-400 border border-blue-600/30 rounded-md text-xs font-medium hover:bg-blue-600/20 shrink-0"
          >
            Review
          </button>
        )}
      </div>
    </div>
  );
}
