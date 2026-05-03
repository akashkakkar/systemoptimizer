import { useEffect } from "react";
import { useMetricsStore } from "../stores/metricsStore";

export function useSystemMetrics() {
  const { disks, memory, cpu, processes, loading, error, lastScan, fetchAll } =
    useMetricsStore();

  useEffect(() => {
    fetchAll();
  }, [fetchAll]);

  return { disks, memory, cpu, processes, loading, error, lastScan, refresh: fetchAll };
}
