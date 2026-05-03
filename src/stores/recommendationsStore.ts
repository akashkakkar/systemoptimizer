import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  Recommendation,
  RecommendationStatus,
  ApprovalResponse,
} from "../types/recommendation";

interface RecommendationsState {
  recommendations: Recommendation[];
  loading: boolean;
  error: string | null;
  filter: RecommendationStatus | "all";
  setFilter: (filter: RecommendationStatus | "all") => void;
  scanSystem: () => Promise<void>;
  fetchRecommendations: () => Promise<void>;
  approveRecommendation: (id: string) => Promise<ApprovalResponse>;
  rejectRecommendation: (id: string, reason?: string) => Promise<void>;
  dismissRecommendation: (id: string) => Promise<void>;
}

export const useRecommendationsStore = create<RecommendationsState>((set, get) => ({
  recommendations: [],
  loading: false,
  error: null,
  filter: "all",

  setFilter: (filter) => set({ filter }),

  scanSystem: async () => {
    set({ loading: true, error: null });
    try {
      await invoke("scan_system");
      await get().fetchRecommendations();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err), loading: false });
    }
  },

  fetchRecommendations: async () => {
    set({ loading: true, error: null });
    try {
      const recs = await invoke<Recommendation[]>("get_recommendations", {
        status: null,
      });
      set({ recommendations: recs, loading: false });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err), loading: false });
    }
  },

  approveRecommendation: async (id: string) => {
    const result = await invoke<ApprovalResponse>("approve_recommendation", { id });
    await get().fetchRecommendations();
    return result;
  },

  rejectRecommendation: async (id: string, reason?: string) => {
    await invoke("reject_recommendation", { id, reason: reason ?? null });
    await get().fetchRecommendations();
  },

  dismissRecommendation: async (id: string) => {
    await invoke("dismiss_recommendation", { id });
    await get().fetchRecommendations();
  },
}));
