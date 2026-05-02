import { useEffect } from "react";
import { useMetricsStore } from "../stores/metricsStore";

export function useSystemMetrics() {
  const { disks, loading, error, lastScan, fetchDiskUsage } =
    useMetricsStore();

  useEffect(() => {
    fetchDiskUsage();
  }, [fetchDiskUsage]);

  return { disks, loading, error, lastScan, refresh: fetchDiskUsage };
}
