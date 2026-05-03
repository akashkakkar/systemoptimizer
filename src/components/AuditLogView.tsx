import { useCallback, useEffect, useState } from "react";
import { useAuditStore } from "../stores/auditStore";
import type {
  ActionResult,
  AuditEntry,
  ExportFormat,
} from "../types/audit";
import type { RiskLevel } from "../types/recommendation";

const RESULT_COLORS: Record<ActionResult, string> = {
  success: "bg-green-600/20 text-green-400 border-green-600/30",
  failed: "bg-red-600/20 text-red-400 border-red-600/30",
  rolled_back: "bg-yellow-600/20 text-yellow-400 border-yellow-600/30",
};

const RISK_COLORS: Record<RiskLevel, string> = {
  critical: "text-red-400",
  high: "text-orange-400",
  medium: "text-yellow-400",
  low: "text-green-400",
};

const STATUS_OPTIONS: Array<{ value: ActionResult | "all"; label: string }> = [
  { value: "all", label: "All" },
  { value: "success", label: "Success" },
  { value: "failed", label: "Failed" },
  { value: "rolled_back", label: "Rolled Back" },
];

export function AuditLogView() {
  const { page, loading, error, filter, setFilter, fetchAuditLog, rollback, exportLog } =
    useAuditStore();

  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [searchInput, setSearchInput] = useState("");
  const [rollingBack, setRollingBack] = useState<string | null>(null);

  useEffect(() => {
    fetchAuditLog();
  }, [fetchAuditLog, filter]);

  const handleSearch = useCallback(() => {
    setFilter({ search: searchInput || undefined, page: 1 });
  }, [searchInput, setFilter]);

  const handleStatusFilter = useCallback(
    (status: ActionResult | "all") => {
      setFilter({
        status: status === "all" ? undefined : status,
        page: 1,
      });
    },
    [setFilter]
  );

  const handleRollback = useCallback(
    async (entry: AuditEntry) => {
      setRollingBack(entry.id);
      try {
        await rollback(entry.id);
      } finally {
        setRollingBack(null);
      }
    },
    [rollback]
  );

  const handleExport = useCallback(
    async (format: ExportFormat) => {
      const ext = format === "json" ? "json" : "csv";
      const now = new Date().toISOString().slice(0, 10);
      const path = `lso-audit-${now}.${ext}`;
      await exportLog(format, path);
    },
    [exportLog]
  );

  const handlePageChange = useCallback(
    (newPage: number) => {
      setFilter({ page: newPage });
    },
    [setFilter]
  );

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold text-gray-100">Audit Log</h2>
        <div className="flex gap-2">
          <button
            onClick={() => handleExport("json")}
            className="px-3 py-1.5 bg-gray-700 text-gray-300 rounded-md text-xs font-medium hover:bg-gray-600 border border-gray-600"
          >
            Export JSON
          </button>
          <button
            onClick={() => handleExport("csv")}
            className="px-3 py-1.5 bg-gray-700 text-gray-300 rounded-md text-xs font-medium hover:bg-gray-600 border border-gray-600"
          >
            Export CSV
          </button>
        </div>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
          {error}
        </div>
      )}

      <div className="flex flex-wrap gap-3 mb-4">
        <div className="flex gap-2" role="tablist">
          {STATUS_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              role="tab"
              aria-selected={
                (filter.status ?? "all") === opt.value
              }
              onClick={() => handleStatusFilter(opt.value)}
              className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
                (filter.status ?? "all") === opt.value
                  ? "bg-blue-600/20 text-blue-400 border border-blue-600/40"
                  : "bg-gray-800 text-gray-400 border border-gray-700 hover:bg-gray-750"
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        <div className="flex gap-2 ml-auto">
          <input
            type="text"
            placeholder="Search target or action…"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            className="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-md text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 w-64"
          />
          <button
            onClick={handleSearch}
            className="px-3 py-1.5 bg-gray-700 text-gray-300 rounded-md text-sm hover:bg-gray-600 border border-gray-600"
          >
            Search
          </button>
        </div>
      </div>

      {loading && !page && (
        <div className="flex items-center justify-center py-16" role="status">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-gray-600 border-t-blue-500" />
          <span className="ml-3 text-gray-400">Loading audit log…</span>
        </div>
      )}

      {page && page.entries.length === 0 && (
        <p className="text-gray-500 text-center py-8">No audit entries found.</p>
      )}

      {page && page.entries.length > 0 && (
        <>
          <div className="space-y-2">
            {page.entries.map((entry) => (
              <AuditEntryRow
                key={entry.id}
                entry={entry}
                expanded={expandedId === entry.id}
                onToggle={() =>
                  setExpandedId(expandedId === entry.id ? null : entry.id)
                }
                onRollback={() => handleRollback(entry)}
                rollingBack={rollingBack === entry.id}
              />
            ))}
          </div>

          <div className="flex items-center justify-between mt-6">
            <span className="text-sm text-gray-500">
              {page.total_count} entries · page {page.page} of {page.total_pages}
            </span>
            <div className="flex gap-2">
              <button
                onClick={() => handlePageChange(page.page - 1)}
                disabled={page.page <= 1}
                className="px-3 py-1.5 bg-gray-800 text-gray-300 rounded-md text-sm disabled:opacity-30 hover:bg-gray-700 border border-gray-700"
              >
                Previous
              </button>
              <button
                onClick={() => handlePageChange(page.page + 1)}
                disabled={page.page >= page.total_pages}
                className="px-3 py-1.5 bg-gray-800 text-gray-300 rounded-md text-sm disabled:opacity-30 hover:bg-gray-700 border border-gray-700"
              >
                Next
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function AuditEntryRow({
  entry,
  expanded,
  onToggle,
  onRollback,
  rollingBack,
}: {
  entry: AuditEntry;
  expanded: boolean;
  onToggle: () => void;
  onRollback: () => void;
  rollingBack: boolean;
}) {
  const ts = new Date(entry.timestamp);

  return (
    <div className="border border-gray-700 rounded-lg bg-gray-800/50">
      <button
        onClick={onToggle}
        className="w-full px-4 py-3 flex items-center gap-3 text-left hover:bg-gray-800/80 transition-colors"
      >
        <span
          className={`px-2 py-0.5 rounded text-xs font-bold border ${RESULT_COLORS[entry.result]}`}
        >
          {entry.result}
        </span>
        <span className="flex-1 text-sm text-gray-200 truncate">
          {entry.action}
        </span>
        <span className="text-xs text-gray-500 truncate max-w-48">
          {entry.target}
        </span>
        <span className={`text-xs font-medium ${RISK_COLORS[entry.risk_level]}`}>
          {entry.risk_level}
        </span>
        <span className="text-xs text-gray-500 tabular-nums whitespace-nowrap">
          {ts.toLocaleDateString()} {ts.toLocaleTimeString()}
        </span>
        <svg
          className={`w-4 h-4 text-gray-500 transition-transform ${expanded ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>

      {expanded && (
        <div className="px-4 pb-3 border-t border-gray-700 pt-3">
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div>
              <span className="text-gray-500">Category:</span>{" "}
              <span className="text-gray-300">{entry.category || "—"}</span>
            </div>
            <div>
              <span className="text-gray-500">Snapshot:</span>{" "}
              <span className="text-gray-300 font-mono">
                {entry.snapshot_id ?? "—"}
              </span>
            </div>
            <div>
              <span className="text-gray-500">User Approved:</span>{" "}
              <span className="text-gray-300">
                {entry.user_approved ? "Yes" : "No"}
              </span>
            </div>
            <div>
              <span className="text-gray-500">ID:</span>{" "}
              <span className="text-gray-300 font-mono text-[10px]">{entry.id}</span>
            </div>
          </div>
          {entry.details && (
            <p className="mt-2 text-xs text-gray-400">{entry.details}</p>
          )}
          {entry.rollback_available && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRollback();
              }}
              disabled={rollingBack}
              className="mt-3 px-3 py-1.5 bg-yellow-600/10 text-yellow-400 border border-yellow-600/30 rounded-md text-xs font-medium hover:bg-yellow-600/20 disabled:opacity-50"
            >
              {rollingBack ? "Rolling back…" : "Rollback"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
