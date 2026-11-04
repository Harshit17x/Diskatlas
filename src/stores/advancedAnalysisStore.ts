import { create } from 'zustand';
import { safeInvoke } from '../utils/tauri';
import { useScanStore } from './scanStore';
import type {
  LargeFilesQuery,
  LargeFilesResponse,
  HiddenFilesQuery,
  HiddenFilesResponse,
  DuplicateSearchQuery,
  DuplicateSearchResponse,
  ForgottenFilesQuery,
  ForgottenFilesResponse,
  UnknownStorageResponse,
} from '../types/bindings';

interface AdvancedAnalysisState {
  // Session selector helper
  selectedSessionId: string | null;
  setSelectedSessionId: (id: string | null) => void;

  // 1. Large Files
  largeFiles: LargeFilesResponse | null;
  largeFilesLoading: boolean;
  largeFilesQuery: LargeFilesQuery;
  setLargeFilesQuery: (query: Partial<LargeFilesQuery>) => void;
  fetchLargeFiles: (sessionId?: string) => Promise<void>;

  // 2. Hidden Files
  hiddenFiles: HiddenFilesResponse | null;
  hiddenFilesLoading: boolean;
  hiddenFilesQuery: HiddenFilesQuery;
  setHiddenFilesQuery: (query: Partial<HiddenFilesQuery>) => void;
  fetchHiddenFiles: (sessionId?: string) => Promise<void>;

  // 3. Duplicate Detection
  duplicateResult: DuplicateSearchResponse | null;
  duplicatesLoading: boolean;
  duplicateQuery: DuplicateSearchQuery;
  setDuplicateQuery: (query: Partial<DuplicateSearchQuery>) => void;
  searchDuplicates: (sessionId?: string) => Promise<void>;

  // 4. Forgotten Files
  forgottenFiles: ForgottenFilesResponse | null;
  forgottenFilesLoading: boolean;
  forgottenFilesQuery: ForgottenFilesQuery;
  setForgottenFilesQuery: (query: Partial<ForgottenFilesQuery>) => void;
  fetchForgottenFiles: (sessionId?: string) => Promise<void>;

  // 5. Unknown Storage
  unknownStorage: UnknownStorageResponse | null;
  unknownStorageLoading: boolean;
  fetchUnknownStorage: (sessionId?: string) => Promise<void>;

  error: string | null;
}

export const useAdvancedAnalysisStore = create<AdvancedAnalysisState>((set, get) => ({
  selectedSessionId: null,
  setSelectedSessionId: (id) => set({ selectedSessionId: id }),
  error: null,

  // 1. Large Files
  largeFiles: null,
  largeFilesLoading: false,
  largeFilesQuery: {
    minSizeBytes: 52428800, // 50 MB
    sortBy: 'size_desc',
    page: 1,
    pageSize: 25,
  },
  setLargeFilesQuery: (query) =>
    set((state) => ({
      largeFilesQuery: { ...state.largeFilesQuery, ...query },
    })),
  fetchLargeFiles: async (sessionId) => {
    set({ largeFilesLoading: true, error: null });
    const sid = sessionId || get().selectedSessionId || useScanStore.getState().activeSessionId || undefined;
    const res = await safeInvoke<LargeFilesResponse>('get_large_files', {
      query: { ...get().largeFilesQuery, sessionId: sid },
    });
    if (res.ok) {
      set({ largeFiles: res.data, largeFilesLoading: false });
    } else {
      set({ error: res.error.message, largeFilesLoading: false });
    }
  },

  // 2. Hidden Files
  hiddenFiles: null,
  hiddenFilesLoading: false,
  hiddenFilesQuery: {
    filterType: 'all',
    search: '',
    page: 1,
    pageSize: 25,
  },
  setHiddenFilesQuery: (query) =>
    set((state) => ({
      hiddenFilesQuery: { ...state.hiddenFilesQuery, ...query },
    })),
  fetchHiddenFiles: async (sessionId) => {
    set({ hiddenFilesLoading: true, error: null });
    const sid = sessionId || get().selectedSessionId || useScanStore.getState().activeSessionId || undefined;
    const res = await safeInvoke<HiddenFilesResponse>('get_hidden_files', {
      query: { ...get().hiddenFilesQuery, sessionId: sid },
    });
    if (res.ok) {
      set({ hiddenFiles: res.data, hiddenFilesLoading: false });
    } else {
      set({ error: res.error.message, hiddenFilesLoading: false });
    }
  },

  // 3. Duplicate Detection
  duplicateResult: null,
  duplicatesLoading: false,
  duplicateQuery: {
    minSizeBytes: 1048576, // 1 MB
    limitGroups: 100,
  },
  setDuplicateQuery: (query) =>
    set((state) => ({
      duplicateQuery: { ...state.duplicateQuery, ...query },
    })),
  searchDuplicates: async (sessionId) => {
    set({ duplicatesLoading: true, error: null });
    const sid = sessionId || get().selectedSessionId || useScanStore.getState().activeSessionId || undefined;
    const res = await safeInvoke<DuplicateSearchResponse>('find_duplicates', {
      query: { ...get().duplicateQuery, sessionId: sid },
    });
    if (res.ok) {
      set({ duplicateResult: res.data, duplicatesLoading: false });
    } else {
      set({ error: res.error.message, duplicatesLoading: false });
    }
  },

  // 4. Forgotten Files
  forgottenFiles: null,
  forgottenFilesLoading: false,
  forgottenFilesQuery: {
    minAgeDays: 90,
    page: 1,
    pageSize: 25,
  },
  setForgottenFilesQuery: (query) =>
    set((state) => ({
      forgottenFilesQuery: { ...state.forgottenFilesQuery, ...query },
    })),
  fetchForgottenFiles: async (sessionId) => {
    set({ forgottenFilesLoading: true, error: null });
    const sid = sessionId || get().selectedSessionId || useScanStore.getState().activeSessionId || undefined;
    const res = await safeInvoke<ForgottenFilesResponse>('get_forgotten_files', {
      query: { ...get().forgottenFilesQuery, sessionId: sid },
    });
    if (res.ok) {
      set({ forgottenFiles: res.data, forgottenFilesLoading: false });
    } else {
      set({ error: res.error.message, forgottenFilesLoading: false });
    }
  },

  // 5. Unknown Storage
  unknownStorage: null,
  unknownStorageLoading: false,
  fetchUnknownStorage: async (sessionId) => {
    set({ unknownStorageLoading: true, error: null });
    const sid = sessionId || get().selectedSessionId || useScanStore.getState().activeSessionId || undefined;
    const res = await safeInvoke<UnknownStorageResponse>('get_unknown_storage', {
      sessionId: sid,
    });
    if (res.ok) {
      set({ unknownStorage: res.data, unknownStorageLoading: false });
    } else {
      set({ error: res.error.message, unknownStorageLoading: false });
    }
  },
}));
