import { create } from 'zustand';
import { safeInvoke } from '../utils/tauri';
import { useScanStore } from './scanStore';
import { useHistoryStore } from './historyStore';
import type {
  HierarchicalNode,
  HierarchicalQuery,
  StorageAnalyticsData,
} from '../types/bindings';

interface VisualizationState {
  // Tree state for Treemap & Sunburst
  tree: HierarchicalNode | null;
  isLoadingTree: boolean;
  currentRootPath: string | null;
  breadcrumbs: string[];
  maxDepth: number;

  // Analytics state
  analyticsData: StorageAnalyticsData | null;
  isLoadingAnalytics: boolean;

  error: string | null;

  // Tree actions
  fetchTree: (sessionId?: string, rootPath?: string, maxDepth?: number) => Promise<void>;
  drillDown: (path: string) => Promise<void>;
  navigateBreadcrumb: (path: string) => Promise<void>;
  navigateUp: () => Promise<void>;
  resetDrilldown: () => Promise<void>;
  setMaxDepth: (depth: number) => void;

  // Analytics action
  fetchAnalytics: (sessionId?: string) => Promise<void>;
}

export const useVisualizationStore = create<VisualizationState>((set, get) => ({
  tree: null,
  isLoadingTree: false,
  currentRootPath: null,
  breadcrumbs: [],
  maxDepth: 3,

  analyticsData: null,
  isLoadingAnalytics: false,
  error: null,

  setMaxDepth: (depth) => {
    set({ maxDepth: depth });
    get().fetchTree(undefined, get().currentRootPath || undefined, depth);
  },

  fetchTree: async (sessionId, rootPath, maxDepthOverride) => {
    let sid = sessionId;
    if (!sid) {
      sid = useScanStore.getState().activeSessionId || useScanStore.getState().summary?.sessionId;
    }
    if (!sid) {
      const recent = useHistoryStore.getState().sessionsPage?.items[0];
      if (recent) {
        sid = recent.id;
      }
    }

    const depth = maxDepthOverride || get().maxDepth;
    set({ isLoadingTree: true, error: null });

    const query: HierarchicalQuery = {
      sessionId: sid || undefined,
      rootPath: rootPath || undefined,
      maxDepth: depth,
    };

    const res = await safeInvoke<HierarchicalNode>('get_hierarchical_tree', { query });

    if (res.ok) {
      const activePath = res.data.path;
      // Reconstruct breadcrumbs from path
      const parts = activePath
        .replace(/\\/g, '/')
        .split('/')
        .filter(Boolean);

      const crumbs: string[] = [];
      let accum = activePath.startsWith('/') ? '' : '';
      for (const part of parts) {
        accum += (accum === '' && !activePath.startsWith('/') ? '' : '/') + part;
        crumbs.push(accum);
      }
      if (crumbs.length === 0) {
        crumbs.push(activePath);
      }

      set({
        tree: res.data,
        currentRootPath: activePath,
        breadcrumbs: crumbs,
        isLoadingTree: false,
      });
    } else {
      set({ error: res.error.message, isLoadingTree: false });
    }
  },

  drillDown: async (path) => {
    await get().fetchTree(undefined, path);
  },

  navigateBreadcrumb: async (path) => {
    await get().fetchTree(undefined, path);
  },

  navigateUp: async () => {
    const { breadcrumbs } = get();
    if (breadcrumbs.length > 1) {
      const parentPath = breadcrumbs[breadcrumbs.length - 2];
      await get().fetchTree(undefined, parentPath);
    } else {
      await get().resetDrilldown();
    }
  },

  resetDrilldown: async () => {
    await get().fetchTree(undefined, undefined);
  },

  fetchAnalytics: async (sessionId) => {
    let sid = sessionId;
    if (!sid) {
      sid = useScanStore.getState().activeSessionId || useScanStore.getState().summary?.sessionId;
    }
    if (!sid) {
      const recent = useHistoryStore.getState().sessionsPage?.items[0];
      if (recent) {
        sid = recent.id;
      }
    }

    set({ isLoadingAnalytics: true, error: null });
    const res = await safeInvoke<StorageAnalyticsData>('get_storage_analytics', {
      sessionId: sid || null,
    });

    if (res.ok) {
      set({ analyticsData: res.data, isLoadingAnalytics: false });
    } else {
      set({ error: res.error.message, isLoadingAnalytics: false });
    }
  },
}));
