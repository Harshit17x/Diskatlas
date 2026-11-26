import { create } from 'zustand';
import { safeInvoke } from '../utils/tauri';
import { resolvePreferredSessionId } from '../utils/session';
import type {
  CleanupPlan,
  ExecuteCleanupRequest,
  ExecuteCleanupResponse,
  CleanupAuditRecord,
  CleanupAuditQuery,
  CleanupAuditResponse,
} from '../types/bindings';

interface CleanupState {
  cleanupPlan: CleanupPlan | null;
  loadingPlan: boolean;
  executingCleanup: boolean;
  executionResponse: ExecuteCleanupResponse | null;
  error: string | null;

  // Selected candidate IDs for cleanup
  selectedIds: Set<string>;

  // Filters
  groupFilter: 'all' | 'safe_to_review' | 'review_required' | 'system_protected';
  riskFilter: 'all' | 'none' | 'low' | 'moderate' | 'high' | 'critical';
  searchQuery: string;

  // Cleanup Options
  useRecycleBin: boolean;
  confirmationModalOpen: boolean;

  // Audit History
  auditRecords: CleanupAuditRecord[];
  auditTotalItems: number;
  auditPage: number;
  auditTotalPages: number;
  auditLoading: boolean;
  auditStatusFilter: string | null;

  // Actions
  fetchCleanupPlan: (sessionId?: string | null) => Promise<void>;
  toggleCandidateSelection: (id: string) => void;
  selectAllSafe: () => void;
  deselectAll: () => void;
  setGroupFilter: (group: 'all' | 'safe_to_review' | 'review_required' | 'system_protected') => void;
  setRiskFilter: (risk: 'all' | 'none' | 'low' | 'moderate' | 'high' | 'critical') => void;
  setSearchQuery: (query: string) => void;
  setUseRecycleBin: (val: boolean) => void;
  setConfirmationModalOpen: (open: boolean) => void;
  executeCleanup: (confirmationPhrase: string) => Promise<boolean>;
  fetchAuditHistory: (page?: number, status?: string) => Promise<void>;
  resetExecutionResponse: () => void;
}

export const useCleanupStore = create<CleanupState>((set, get) => ({
  cleanupPlan: null,
  loadingPlan: false,
  executingCleanup: false,
  executionResponse: null,
  error: null,

  selectedIds: new Set<string>(),

  groupFilter: 'all',
  riskFilter: 'all',
  searchQuery: '',

  useRecycleBin: true,
  confirmationModalOpen: false,

  auditRecords: [],
  auditTotalItems: 0,
  auditPage: 1,
  auditTotalPages: 1,
  auditLoading: false,
  auditStatusFilter: null,

  fetchCleanupPlan: async (sessionId?: string | null) => {
    const sid = resolvePreferredSessionId(sessionId);
    set({ loadingPlan: true, error: null });
    const res = await safeInvoke<CleanupPlan>('get_cleanup_plan', {
      sessionId: sid || null,
    });
    if (res.ok) {
      const initialSelected = new Set<string>();
      res.data.candidates.forEach((c) => {
        if (c.selected && !c.isBlocked) {
          initialSelected.add(c.id);
        }
      });

      set({
        cleanupPlan: res.data,
        selectedIds: initialSelected,
        loadingPlan: false,
      });
    } else {
      set({
        error: res.error.message,
        loadingPlan: false,
      });
    }
  },

  toggleCandidateSelection: (id: string) => {
    const { selectedIds, cleanupPlan } = get();
    if (!cleanupPlan) return;

    const candidate = cleanupPlan.candidates.find((c) => c.id === id);
    if (!candidate || candidate.isBlocked) return;

    const nextSelected = new Set(selectedIds);
    if (nextSelected.has(id)) {
      nextSelected.delete(id);
    } else {
      nextSelected.add(id);
    }
    set({ selectedIds: nextSelected });
  },

  selectAllSafe: () => {
    const { cleanupPlan } = get();
    if (!cleanupPlan) return;

    const nextSelected = new Set<string>();
    cleanupPlan.candidates.forEach((c) => {
      if (!c.isBlocked && c.group === 'safe_to_review') {
        nextSelected.add(c.id);
      }
    });
    set({ selectedIds: nextSelected });
  },

  deselectAll: () => {
    set({ selectedIds: new Set() });
  },

  setGroupFilter: (group) => set({ groupFilter: group }),
  setRiskFilter: (risk) => set({ riskFilter: risk }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setUseRecycleBin: (val) => set({ useRecycleBin: val }),
  setConfirmationModalOpen: (open) => set({ confirmationModalOpen: open }),

  executeCleanup: async (confirmationPhrase: string) => {
    const { cleanupPlan, selectedIds, useRecycleBin, fetchAuditHistory, fetchCleanupPlan } = get();
    if (!cleanupPlan || selectedIds.size === 0) return false;

    set({ executingCleanup: true, error: null });
    const request: ExecuteCleanupRequest = {
      batchId: cleanupPlan.batchId,
      sessionId: cleanupPlan.sessionId,
      itemIds: Array.from(selectedIds),
      useRecycleBin,
      confirmationPhrase,
    };

    const res = await safeInvoke<ExecuteCleanupResponse>('execute_cleanup', { request });
    if (res.ok) {
      set({
        executionResponse: res.data,
        executingCleanup: false,
        confirmationModalOpen: false,
      });

      await fetchAuditHistory(1);
      await fetchCleanupPlan(cleanupPlan.sessionId);
      import('./driveStore').then(({ useDriveStore }) => {
        useDriveStore.getState().fetchDbHealth();
      });
      return true;
    }

    set({
      error: res.error.message,
      executingCleanup: false,
    });
    return false;
  },

  fetchAuditHistory: async (page = 1, status) => {
    set({ auditLoading: true });
    const query: CleanupAuditQuery = {
      page,
      pageSize: 15,
      status: status || undefined,
    };
    const res = await safeInvoke<CleanupAuditResponse>('get_cleanup_audit_history', { query });
    if (res.ok) {
      set({
        auditRecords: res.data.items,
        auditTotalItems: res.data.totalItems,
        auditPage: res.data.page,
        auditTotalPages: res.data.totalPages,
        auditLoading: false,
        auditStatusFilter: status || null,
      });
    } else {
      set({ auditLoading: false, error: res.error.message });
    }
  },

  resetExecutionResponse: () => set({ executionResponse: null }),
}));
