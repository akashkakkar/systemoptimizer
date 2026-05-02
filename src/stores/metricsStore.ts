import { create } from "zustand";
import { type DiskUsageReport, getDiskUsage } from "../lib/tauri";

interface MetricsState {
  disks: DiskUsageReport[];
  loading: boolean;
  error: string | null;
  lastScan: Date | null;
  fetchDiskUsage: () => Promise<void>;
}

export const useMetricsStore = create<MetricsState>((set) => ({
  disks: [],
  loading: false,
  error: null,
  lastScan: null,

  fetchDiskUsage: async () => {
    set({ loading: true, error: null });
    try {
      const disks = await getDiskUsage();
      set({ disks, loading: false, lastScan: new Date() });
    } catch (err) {
      set({
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },
}));
