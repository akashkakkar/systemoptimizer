import { useSystemMetrics } from "../hooks/useSystemMetrics";
import { CpuCard } from "./CpuCard";
import { DiskUsageCard } from "./DiskUsageCard";
import { MemoryCard } from "./MemoryCard";
import { ProcessTable } from "./ProcessTable";
import { RecommendationList } from "./RecommendationList";

export function DashboardView() {
  const { disks, memory, cpu, processes, loading, error, lastScan, refresh } =
    useSystemMetrics();

  return (
    <main className="max-w-5xl mx-auto px-6 py-8">
      <header className="mb-8">
        <h1 className="text-2xl font-bold text-gray-100">
          Local System Optimizer
        </h1>
        <p className="text-gray-400 mt-1">System health overview</p>
      </header>

      <div className="flex items-center gap-4 mb-6">
        <button
          onClick={refresh}
          disabled={loading}
          aria-label="Scan system now"
          className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-blue-400 focus:ring-offset-2 focus:ring-offset-gray-950"
        >
          {loading ? "Scanning…" : "Scan Now"}
        </button>

        {lastScan && (
          <span className="text-sm text-gray-500">
            Last scan: {lastScan.toLocaleTimeString()}
          </span>
        )}
      </div>

      {loading && disks.length === 0 && !memory && !cpu && (
        <div className="flex items-center justify-center py-16" role="status">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-gray-600 border-t-blue-500" />
          <span className="ml-3 text-gray-400">Scanning system…</span>
        </div>
      )}

      {error && (
        <div
          role="alert"
          className="bg-red-900/30 border border-red-700 text-red-300 rounded-lg p-4 mb-6"
        >
          <p className="font-medium">Scan failed</p>
          <p className="text-sm mt-1">{error}</p>
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2 mb-6">
        {memory && <MemoryCard memory={memory} />}
        {cpu && <CpuCard cpu={cpu} />}
      </div>

      {disks.length > 0 && (
        <div className="grid gap-4 sm:grid-cols-1 md:grid-cols-2 mb-6">
          {disks.map((disk) => (
            <DiskUsageCard key={disk.mount_point} disk={disk} />
          ))}
        </div>
      )}

      {processes.length > 0 && (
        <div className="mb-6">
          <ProcessTable processes={processes} />
        </div>
      )}

      <div className="border-t border-gray-800 pt-6">
        <RecommendationList />
      </div>
    </main>
  );
}
