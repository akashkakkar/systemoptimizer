import type { CpuInfo } from "../lib/tauri";
import { MetricBar } from "./MetricBar";

interface CpuCardProps {
  cpu: CpuInfo;
}

export function CpuCard({ cpu }: CpuCardProps) {
  const avgUsage =
    cpu.per_core_percent.length > 0
      ? cpu.per_core_percent.reduce((a, b) => a + b, 0) /
        cpu.per_core_percent.length
      : 0;

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5">
      <h3 className="text-lg font-semibold text-gray-100 mb-4">
        CPU ({cpu.core_count} cores)
      </h3>

      <MetricBar percent={avgUsage} label="Average CPU" />

      <div className="grid grid-cols-3 gap-3 mt-4 text-sm">
        <div>
          <span className="text-gray-500">Load 1m</span>
          <p className="text-gray-200 font-medium">
            {cpu.load_avg_1.toFixed(2)}
          </p>
        </div>
        <div>
          <span className="text-gray-500">Load 5m</span>
          <p className="text-gray-200 font-medium">
            {cpu.load_avg_5.toFixed(2)}
          </p>
        </div>
        <div>
          <span className="text-gray-500">Load 15m</span>
          <p className="text-gray-200 font-medium">
            {cpu.load_avg_15.toFixed(2)}
          </p>
        </div>
      </div>

      {cpu.per_core_percent.length > 0 && (
        <div className="mt-4 space-y-2">
          {cpu.per_core_percent.map((pct, i) => (
            <MetricBar key={i} percent={pct} label={`Core ${i}`} />
          ))}
        </div>
      )}
    </div>
  );
}
