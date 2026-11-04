import { create } from 'zustand';
import type {
  DeveloperAnalysisProfile,
  DeveloperProjectItem,
  DockerStorageInfo,
  GlobalToolCacheItem,
} from '../types/bindings';
import { safeInvoke } from '../utils/tauri';
import { useScanStore } from './scanStore';
import { useHistoryStore } from './historyStore';

interface DeveloperState {
  profile: DeveloperAnalysisProfile | null;
  docker: DockerStorageInfo | null;
  globalCaches: GlobalToolCacheItem[];
  selectedProject: DeveloperProjectItem | null;
  filterEcosystem: string | null;
  isLoading: boolean;
  error: string | null;

  fetchDeveloperStorage: (sessionIdOverride?: string) => Promise<void>;
  fetchDockerStorage: () => Promise<void>;
  fetchGlobalCaches: () => Promise<void>;
  setFilterEcosystem: (ecosystem: string | null) => void;
  setSelectedProject: (project: DeveloperProjectItem | null) => void;
}

export const useDeveloperStore = create<DeveloperState>((set, get) => ({
  profile: null,
  docker: null,
  globalCaches: [],
  selectedProject: null,
  filterEcosystem: null,
  isLoading: false,
  error: null,

  setFilterEcosystem: (ecosystem) => set({ filterEcosystem: ecosystem }),
  setSelectedProject: (project) => set({ selectedProject: project }),

  fetchDeveloperStorage: async (sessionIdOverride) => {
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
      // Still fetch live Docker and global caches even without a scan session
      get().fetchDockerStorage();
      get().fetchGlobalCaches();
      return;
    }

    set({ isLoading: true, error: null });
    const res = await safeInvoke<DeveloperAnalysisProfile>('get_developer_storage', {
      sessionId: sid,
    });

    if (res.ok) {
      set({
        profile: res.data,
        docker: res.data.docker,
        globalCaches: res.data.globalCaches,
        isLoading: false,
      });
    } else {
      set({ isLoading: false, error: res.error.message });
      get().fetchDockerStorage();
      get().fetchGlobalCaches();
    }
  },

  fetchDockerStorage: async () => {
    const res = await safeInvoke<DockerStorageInfo>('get_docker_storage');
    if (res.ok) {
      set({ docker: res.data });
    }
  },

  fetchGlobalCaches: async () => {
    const res = await safeInvoke<GlobalToolCacheItem[]>('get_global_tool_caches');
    if (res.ok) {
      set({ globalCaches: res.data });
    }
  },
}));
