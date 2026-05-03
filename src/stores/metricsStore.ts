import { create } from "zustand";
import {
  type CpuInfo,
  type DiskUsageReport,
  type MemoryInfo,
  type ProcessInfo,
  getCpuUsage,
  getDiskUsage,
  getMemoryUsage,
  getProcessList,
} from "../lib/tauri";

interface MetricsState {
  disks: DiskUsageReport[];
  memory: MemoryInfo | null;
  cpu: CpuInfo | null;
  processes: ProcessInfo[];
  loading: boolean;
  error: string | null;
  lastScan: Date | null;
  fetchAll: () => Promise<void>;
  fetchDiskUsage: () => Promise<void>;
  fetchMemory: () => Promise<void>;
  fetchCpu: () => Promise<void>;
  fetchProcesses: () => Promise<void>;
}

export const useMetricsStore = create<MetricsState>((set) => ({
  disks: [],
  memory: null,
  cpu: null,
  processes: [],
  loading: false,
  error: null,
  lastScan: null,

  fetchAll: async () => {
    set({ loading: true, error: null });
    try {
      const [disks, memory, cpu, processes] = await Promise.all([
        getDiskUsage(),
        getMemoryUsage().catch(() => null),
        getCpuUsage().catch(() => null),
        getProcessList().catch(() => []),
      ]);
      set({ disks, memory, cpu, processes, loading: false, lastScan: new Date() });
    } catch (err) {
      set({
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

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

  fetchMemory: async () => {
    try {
      const memory = await getMemoryUsage();
      set({ memory });
    } catch (_) {
      /* platform not supported */
    }
  },

  fetchCpu: async () => {
    try {
      const cpu = await getCpuUsage();
      set({ cpu });
    } catch (_) {
      /* platform not supported */
    }
  },

  fetchProcesses: async () => {
    try {
      const processes = await getProcessList();
      set({ processes });
    } catch (_) {
      /* platform not supported */
    }
  },
}));
