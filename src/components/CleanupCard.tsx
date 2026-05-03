import { useCallback, useState } from "react";
import { useAuditStore } from "../stores/auditStore";
import type { CleanupResult, CleanupTarget, PreflightReport } from "../types/audit";

const TARGETS: Array<{ value: CleanupTarget; label: string; desc: string }> = [
  {
    value: "system_temp",
    label: "System Temp",
    desc: "Clean /tmp and system temp directories",
  },
  {
    value: "user_cache",
    label: "User Cache",
    desc: "Clean ~/.cache and app cache directories",
  },
  {
    value: "app_logs",
    label: "App Logs",
    desc: "Remove application logs older than 30 days",
  },
];

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function CleanupCard() {
  const { preflightCleanup, executeCleanup } = useAuditStore();
  const [preflight, setPreflight] = useState<PreflightReport | null>(null);
  const [result, setResult] = useState<CleanupResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stage, setStage] = useState<"select" | "preflight" | "result">("select");

  const handlePreflight = useCallback(
    async (target: CleanupTarget) => {
      setLoading(true);
      setError(null);
      try {
        const report = await preflightCleanup(target);
        setPreflight(report);
        setStage("preflight");
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    },
    [preflightCleanup]
  );

  const handleExecute = useCallback(async () => {
    if (!preflight) return;
    setLoading(true);
    setError(null);
    try {
      const res = await executeCleanup(preflight.target);
      setResult(res);
      setStage("result");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [preflight, executeCleanup]);

  const handleReset = useCallback(() => {
    setPreflight(null);
    setResult(null);
    setError(null);
    setStage("select");
  }, []);

  return (
    <div className="border border-gray-700 rounded-lg bg-gray-800/50 p-5">
      <h3 className="text-lg font-semibold text-gray-100 mb-4">
        Temp File Cleanup
      </h3>

      {error && (
        <div className="mb-4 p-3 bg-red-900/50 border border-red-700 rounded-lg text-red-200 text-sm">
          {error}
        </div>
      )}

      {stage === "select" && (
        <div className="space-y-3">
          {TARGETS.map((t) => (
            <button
              key={t.value}
              onClick={() => handlePreflight(t.value)}
              disabled={loading}
              className="w-full text-left p-3 bg-gray-700/50 border border-gray-600 rounded-lg hover:bg-gray-700 transition-colors disabled:opacity-50"
            >
              <div className="font-medium text-gray-200 text-sm">
                {t.label}
              </div>
              <div className="text-xs text-gray-400 mt-0.5">{t.desc}</div>
            </button>
          ))}
        </div>
      )}

      {stage === "preflight" && preflight && (
        <div>
          <div className="bg-gray-900 rounded-lg p-4 mb-4">
            <h4 className="text-sm font-medium text-gray-300 mb-3">
              Preflight Report
            </h4>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-gray-500">Files to delete:</span>{" "}
                <span className="text-gray-200 font-medium">
                  {preflight.file_count}
                </span>
              </div>
              <div>
                <span className="text-gray-500">Space to reclaim:</span>{" "}
                <span className="text-gray-200 font-medium">
                  {formatBytes(preflight.total_bytes)}
                </span>
              </div>
              {preflight.oldest_modified && (
                <div>
                  <span className="text-gray-500">Oldest file:</span>{" "}
                  <span className="text-gray-200">
                    {new Date(preflight.oldest_modified).toLocaleDateString()}
                  </span>
                </div>
              )}
              {preflight.skipped.length > 0 && (
                <div>
                  <span className="text-gray-500">Skipped:</span>{" "}
                  <span className="text-yellow-400">
                    {preflight.skipped.length} files
                  </span>
                </div>
              )}
            </div>
            {preflight.skipped.length > 0 && (
              <details className="mt-3">
                <summary className="text-xs text-gray-500 cursor-pointer hover:text-gray-400">
                  Show skipped files
                </summary>
                <ul className="mt-2 space-y-1 text-xs text-gray-400 max-h-32 overflow-y-auto">
                  {preflight.skipped.map((s, i) => (
                    <li key={i} className="truncate">
                      <span className="text-gray-500">{s.reason}:</span>{" "}
                      <span className="font-mono">{s.path}</span>
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </div>

          <div className="flex gap-2">
            <button
              onClick={handleExecute}
              disabled={loading || preflight.file_count === 0}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-500 disabled:opacity-50 text-sm font-medium"
            >
              {loading ? "Cleaning…" : "Clean Now"}
            </button>
            <button
              onClick={handleReset}
              disabled={loading}
              className="px-4 py-2 bg-gray-700 text-gray-300 rounded-lg hover:bg-gray-600 text-sm"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {stage === "result" && result && (
        <div>
          <div className="bg-green-900/20 border border-green-700/30 rounded-lg p-4 mb-4">
            <h4 className="text-sm font-medium text-green-400 mb-2">
              Cleanup Complete
            </h4>
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div>
                <span className="text-gray-500">Files deleted:</span>{" "}
                <span className="text-gray-200">{result.files_deleted}</span>
              </div>
              <div>
                <span className="text-gray-500">Space reclaimed:</span>{" "}
                <span className="text-gray-200">
                  {formatBytes(result.bytes_reclaimed)}
                </span>
              </div>
              <div>
                <span className="text-gray-500">Duration:</span>{" "}
                <span className="text-gray-200">{result.duration_ms}ms</span>
              </div>
              <div>
                <span className="text-gray-500">Snapshot:</span>{" "}
                <span className="text-gray-200 font-mono text-xs">
                  {result.snapshot_id}
                </span>
              </div>
            </div>
            {result.errors.length > 0 && (
              <div className="mt-2 text-xs text-red-400">
                {result.errors.length} errors occurred during cleanup
              </div>
            )}
          </div>
          <button
            onClick={handleReset}
            className="px-4 py-2 bg-gray-700 text-gray-300 rounded-lg hover:bg-gray-600 text-sm"
          >
            Done
          </button>
        </div>
      )}
    </div>
  );
}
