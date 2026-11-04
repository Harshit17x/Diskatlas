import { create } from 'zustand';
import type {
  CategoryOverrideRule,
  ScanFilePage,
  StorageAnalysisProfile,
} from '../types/bindings';
import { safeInvoke } from '../utils/tauri';
import { useScanStore } from './scanStore';
import { useHistoryStore } from './historyStore';

interface AnalysisState {
  activeProfile: StorageAnalysisProfile | null;
  selectedCategory: string | null;
  categoryFilePage: ScanFilePage | null;
  currentPage: number;
  pageSize: number;
  overrides: CategoryOverrideRule[];
  isLoading: boolean;
  isLoadingFiles: boolean;
  error: string | null;

  fetchAnalysis: (sessionId?: string) => Promise<void>;
  runClassification: (sessionId?: string) => Promise<void>;
  fetchCategoryFiles: (sessionId?: string, category?: string, page?: number, pageSize?: number) => Promise<void>;
  selectCategory: (category: string | null) => void;
  fetchOverrides: () => Promise<void>;
  addOverride: (pattern: string, category: string, subcategory?: string) => Promise<boolean>;
  deleteOverride: (id: number) => Promise<boolean>;
}

export const useAnalysisStore = create<AnalysisState>((set, get) => ({
  activeProfile: null,
  selectedCategory: null,
  categoryFilePage: null,
  currentPage: 1,
  pageSize: 50,
  overrides: [],
  isLoading: false,
  isLoadingFiles: false,
  error: null,

  selectCategory: (category: string | null) => {
    set({ selectedCategory: category, currentPage: 1, categoryFilePage: null });
    if (category) {
      get().fetchCategoryFiles(undefined, category, 1, get().pageSize);
    }
  },

  fetchAnalysis: async (sessionIdOverride) => {
    // Resolve target session ID: either passed in, active in scanStore, or most recent in historyStore
    let sid = sessionIdOverride;
    if (!sid) {
      sid = useScanStore.getState().activeSessionId || useScanStore.getState().summary?.sessionId;
    }
    if (!sid) {
      const recent = useHistoryStore.getState().sessionsPage?.items[0];
      if (recent) {
        sid = recent.id;
      }
    }

    if (!sid) {
      return;
    }

    set({ isLoading: true, error: null });
    const res = await safeInvoke<StorageAnalysisProfile>('get_storage_analysis', {
      sessionId: sid,
    });

    if (res.ok) {
      set({
        activeProfile: res.data,
        isLoading: false,
      });
    } else {
      set({ isLoading: false, error: res.error.message });
    }
  },

  runClassification: async (sessionIdOverride) => {
    let sid = sessionIdOverride;
    if (!sid) {
      sid = get().activeProfile?.sessionId || useScanStore.getState().activeSessionId || undefined;
    }
    if (!sid) return;

    set({ isLoading: true, error: null });
    const res = await safeInvoke<StorageAnalysisProfile>('classify_session', {
      sessionId: sid,
    });

    if (res.ok) {
      set({ activeProfile: res.data, isLoading: false });
    } else {
      set({ isLoading: false, error: res.error.message });
    }
  },

  fetchCategoryFiles: async (sessionIdOverride, categoryOverride, page = 1, pageSize = 50) => {
    const sid = sessionIdOverride || get().activeProfile?.sessionId;
    const cat = categoryOverride || get().selectedCategory;
    if (!sid || !cat) return;

    set({ isLoadingFiles: true });
    const res = await safeInvoke<ScanFilePage>('get_category_files', {
      sessionId: sid,
      category: cat,
      page,
      pageSize,
    });

    if (res.ok) {
      set({
        categoryFilePage: res.data,
        currentPage: page,
        pageSize,
        isLoadingFiles: false,
      });
    } else {
      set({ isLoadingFiles: false, error: res.error.message });
    }
  },

  fetchOverrides: async () => {
    const res = await safeInvoke<CategoryOverrideRule[]>('get_category_overrides');
    if (res.ok) {
      set({ overrides: res.data });
    }
  },

  addOverride: async (pattern: string, category: string, subcategory?: string) => {
    const res = await safeInvoke<CategoryOverrideRule>('add_category_override', {
      pattern,
      category,
      subcategory: subcategory || null,
    });
    if (res.ok) {
      get().fetchOverrides();
      return true;
    }
    return false;
  },

  deleteOverride: async (id: number) => {
    const res = await safeInvoke('delete_category_override', { id });
    if (res.ok) {
      get().fetchOverrides();
      return true;
    }
    return false;
  },
}));
