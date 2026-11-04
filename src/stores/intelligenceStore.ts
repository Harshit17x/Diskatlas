import { create } from 'zustand';
import { safeInvoke } from '../utils/tauri';
import { useScanStore } from './scanStore';
import { useHistoryStore } from './historyStore';
import type {
  StorageIntelligenceProfile,
  RecommendationsResponse,
  RecommendationsQuery,
} from '../types/bindings';

interface IntelligenceState {
  selectedSessionId: string | null;
  setSelectedSessionId: (id: string | null) => void;

  profile: StorageIntelligenceProfile | null;
  isLoadingProfile: boolean;

  recommendations: RecommendationsResponse | null;
  isLoadingRecommendations: boolean;

  query: RecommendationsQuery;
  setQuery: (query: Partial<RecommendationsQuery>) => void;

  error: string | null;

  fetchProfile: (sessionId?: string | null) => Promise<void>;
  fetchRecommendations: (sessionId?: string | null) => Promise<void>;
}

export const useIntelligenceStore = create<IntelligenceState>((set, get) => ({
  selectedSessionId: null,
  setSelectedSessionId: (id) => set({ selectedSessionId: id }),

  profile: null,
  isLoadingProfile: false,

  recommendations: null,
  isLoadingRecommendations: false,

  query: {
    group: 'all',
    riskLevel: 'all',
    search: '',
    page: 1,
    pageSize: 20,
  },

  setQuery: (newQuery) =>
    set((state) => ({
      query: { ...state.query, ...newQuery },
    })),

  error: null,

  fetchProfile: async (sessionId) => {
    let sid: string | undefined = sessionId || get().selectedSessionId || undefined;
    if (!sid) {
      sid = useScanStore.getState().activeSessionId || useScanStore.getState().summary?.sessionId || undefined;
    }
    if (!sid) {
      const recent = useHistoryStore.getState().sessionsPage?.items[0];
      if (recent) {
        sid = recent.id;
      }
    }

    set({ isLoadingProfile: true, error: null });
    const res = await safeInvoke<StorageIntelligenceProfile>('get_storage_intelligence', {
      sessionId: sid || null,
    });

    if (res.ok) {
      set({ profile: res.data, isLoadingProfile: false });
      // Automatically fetch initial recommendations for this profile
      get().fetchRecommendations(sid);
    } else {
      set({ error: res.error.message, isLoadingProfile: false });
    }
  },

  fetchRecommendations: async (sessionId) => {
    let sid: string | undefined = sessionId || get().selectedSessionId || get().profile?.sessionId || undefined;
    if (!sid) {
      sid = useScanStore.getState().activeSessionId || useScanStore.getState().summary?.sessionId || undefined;
    }
    if (!sid) {
      const recent = useHistoryStore.getState().sessionsPage?.items[0];
      if (recent) {
        sid = recent.id;
      }
    }

    set({ isLoadingRecommendations: true, error: null });
    const res = await safeInvoke<RecommendationsResponse>('get_storage_recommendations', {
      query: {
        ...get().query,
        sessionId: sid || null,
      },
    });

    if (res.ok) {
      set({ recommendations: res.data, isLoadingRecommendations: false });
    } else {
      set({ error: res.error.message, isLoadingRecommendations: false });
    }
  },
}));
