import type { DiskUsageReport } from "../lib/tauri";
import { MetricBar } from "./MetricBar";

interface DiskUsageCardProps {
  disk: DiskUsageReport;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(1)} ${units[i]}`;
}

export function DiskUsageCard({ disk }: DiskUsageCardProps) {
  return (
    <div className="bg-gray-800 rounded-lg p-5 border border-gray-700">
      <div className="flex items-baseline justify-between mb-3">
        <h3 className="text-lg font-semibold text-gray-100 truncate">
          {disk.mount_point}
        </h3>
        <span className="text-sm text-gray-400 ml-2 shrink-0">
          {disk.fs_type}
        </span>
      </div>

      <MetricBar percent={disk.usage_percent} label="Disk Usage" />

      <div className="grid grid-cols-3 gap-2 mt-3 text-sm text-gray-400">
        <div>
          <span className="block text-gray-500">Used</span>
          {formatBytes(disk.used_bytes)}
        </div>
        <div>
          <span className="block text-gray-500">Available</span>
          {formatBytes(disk.available_bytes)}
        </div>
        <div>
          <span className="block text-gray-500">Total</span>
          {formatBytes(disk.total_bytes)}
        </div>
      </div>
    </div>
  );
}
