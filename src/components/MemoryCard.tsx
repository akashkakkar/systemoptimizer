import type { MemoryInfo } from "../lib/tauri";
import { MetricBar } from "./MetricBar";

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

interface MemoryCardProps {
  memory: MemoryInfo;
}

export function MemoryCard({ memory }: MemoryCardProps) {
  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5">
      <h3 className="text-lg font-semibold text-gray-100 mb-4">Memory</h3>

      <MetricBar percent={memory.usage_percent} label="RAM Usage" />

      <div className="grid grid-cols-3 gap-3 mt-4 text-sm">
        <div>
          <span className="text-gray-500">Total</span>
          <p className="text-gray-200 font-medium">
            {formatBytes(memory.total_bytes)}
          </p>
        </div>
        <div>
          <span className="text-gray-500">Used</span>
          <p className="text-gray-200 font-medium">
            {formatBytes(memory.used_bytes)}
          </p>
        </div>
        <div>
          <span className="text-gray-500">Available</span>
          <p className="text-gray-200 font-medium">
            {formatBytes(memory.available_bytes)}
          </p>
        </div>
      </div>

      {memory.swap_total_bytes > 0 && (
        <div className="mt-4">
          <MetricBar percent={memory.swap_percent} label="Swap Usage" />
          <div className="grid grid-cols-2 gap-3 mt-2 text-sm">
            <div>
              <span className="text-gray-500">Swap Total</span>
              <p className="text-gray-200 font-medium">
                {formatBytes(memory.swap_total_bytes)}
              </p>
            </div>
            <div>
              <span className="text-gray-500">Swap Used</span>
              <p className="text-gray-200 font-medium">
                {formatBytes(memory.swap_used_bytes)}
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
