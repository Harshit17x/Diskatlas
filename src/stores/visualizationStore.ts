import { create } from 'zustand';
import { safeInvoke } from '../utils/tauri';
import { resolvePreferredSessionId } from '../utils/session';
import type {
  HierarchicalNode,
  HierarchicalQuery,
  StorageAnalyticsData,
} from '../types/bindings';

function buildBreadcrumbs(activePath: string, rootAnchor: string | null): string[] {
  const isWindows = /^[A-Za-z]:/.test(activePath) || activePath.includes('\\');
  const sep = isWindows ? '\\' : '/';
  const normActive = activePath.replace(/[\\/]+$/, '') || (isWindows ? activePath.slice(0, 3) : '/');
  const normAnchor = rootAnchor
    ? rootAnchor.replace(/[\\/]+$/, '') || (isWindows ? rootAnchor.slice(0, 3) : '/')
    : null;

  if (normAnchor && (normActive === normAnchor || normActive.startsWith(normAnchor + sep) || normAnchor === '/')) {
    if (normActive === normAnchor) {
      return [rootAnchor!];
    }
    const baseParts = normAnchor === '/' ? [] : normAnchor.split(/[\\/]/).filter(Boolean);
    const activeParts = normActive.split(/[\\/]/).filter(Boolean);
    const crumbs: string[] = [rootAnchor!];
    let current = normAnchor === '/' ? '' : normAnchor;
    for (let i = baseParts.length; i < activeParts.length; i++) {
      current = current ? `${current}${sep}${activeParts[i]}` : `${sep}${activeParts[i]}`;
      crumbs.push(current);
    }
    return crumbs;
  }

  // Standalone full-path reconstruction (supports Unix `/` and Windows `C:\`)
  const parts = activePath.split(/[\\/]/).filter(Boolean);
  if (parts.length === 0) return [activePath];
  if (isWindows) {
    const driveRoot = `${parts[0]}\\`;
    const crumbs = [driveRoot];
    let cur = parts[0];
    for (let i = 1; i < parts.length; i++) {
      cur = `${cur}\\${parts[i]}`;
      crumbs.push(cur);
    }
    return crumbs;
  }

  const crumbs = ['/'];
  let cur = '';
  for (const part of parts) {
    cur += `/${part}`;
    crumbs.push(cur);
  }
  return crumbs;
}

interface VisualizationState {
  // Tree state for Treemap & Sunburst
  tree: HierarchicalNode | null;
  isLoadingTree: boolean;
  currentSessionId: string | null;
  sessionRootPath: string | null;
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
  currentSessionId: null,
  sessionRootPath: null,
  currentRootPath: null,
  breadcrumbs: [],
  maxDepth: 3,

  analyticsData: null,
  isLoadingAnalytics: false,
  error: null,

  setMaxDepth: (depth) => {
    set({ maxDepth: depth });
    get().fetchTree(
      get().currentSessionId || undefined,
      get().currentRootPath || undefined,
      depth
    );
  },

  fetchTree: async (sessionId, rootPath, maxDepthOverride) => {
    const sid = resolvePreferredSessionId(sessionId);
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
      const resolvedSid = res.data.sessionId || sid || null;
      const sessionChanged = resolvedSid !== get().currentSessionId;
      const rootAnchor = !rootPath || sessionChanged ? activePath : get().sessionRootPath || activePath;
      const crumbs = buildBreadcrumbs(activePath, rootAnchor);

      set({
        tree: res.data,
        currentSessionId: resolvedSid,
        sessionRootPath: rootAnchor,
        currentRootPath: activePath,
        breadcrumbs: crumbs,
        isLoadingTree: false,
      });
    } else {
      set({ error: res.error.message, isLoadingTree: false });
    }
  },

  drillDown: async (path) => {
    await get().fetchTree(get().currentSessionId || undefined, path);
  },

  navigateBreadcrumb: async (path) => {
    await get().fetchTree(get().currentSessionId || undefined, path);
  },

  navigateUp: async () => {
    const { breadcrumbs, currentSessionId } = get();
    if (breadcrumbs.length > 1) {
      const parentPath = breadcrumbs[breadcrumbs.length - 2];
      await get().fetchTree(currentSessionId || undefined, parentPath);
    } else {
      await get().resetDrilldown();
    }
  },

  resetDrilldown: async () => {
    await get().fetchTree(get().currentSessionId || undefined, undefined);
  },

  fetchAnalytics: async (sessionId) => {
    const sid = resolvePreferredSessionId(sessionId);

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

