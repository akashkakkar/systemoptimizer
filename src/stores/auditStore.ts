import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  AuditFilter,
  AuditPage,
  CleanupResult,
  CleanupTarget,
  ExportFormat,
  PreflightReport,
} from "../types/audit";

interface AuditState {
  page: AuditPage | null;
  loading: boolean;
  error: string | null;
  filter: AuditFilter;

  setFilter: (filter: Partial<AuditFilter>) => void;
  fetchAuditLog: () => Promise<void>;
  rollback: (auditId: string) => Promise<void>;
  exportLog: (format: ExportFormat, path: string) => Promise<void>;
  preflightCleanup: (target: CleanupTarget) => Promise<PreflightReport>;
  executeCleanup: (target: CleanupTarget) => Promise<CleanupResult>;
}

export const useAuditStore = create<AuditState>((set, get) => ({
  page: null,
  loading: false,
  error: null,
  filter: { page: 1, per_page: 50 },

  setFilter: (partial) => {
    set((state) => ({
      filter: { ...state.filter, ...partial },
    }));
  },

  fetchAuditLog: async () => {
    set({ loading: true, error: null });
    try {
      const page = await invoke<AuditPage>("get_audit_log", {
        filter: get().filter,
      });
      set({ page, loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  rollback: async (auditId: string) => {
    set({ loading: true, error: null });
    try {
      await invoke("rollback_action", { auditId });
      await get().fetchAuditLog();
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  exportLog: async (format: ExportFormat, path: string) => {
    try {
      await invoke("export_audit_log", {
        format,
        path,
        filter: get().filter,
      });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  preflightCleanup: async (target: CleanupTarget) => {
    return invoke<PreflightReport>("preflight_cleanup", { target });
  },

  executeCleanup: async (target: CleanupTarget) => {
    const result = await invoke<CleanupResult>("execute_cleanup", { target });
    await get().fetchAuditLog();
    return result;
  },
}));
