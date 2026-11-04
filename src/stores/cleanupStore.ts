import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
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
  fetchCleanupPlan: (sessionId: string) => Promise<void>;
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

  fetchCleanupPlan: async (sessionId: string) => {
    set({ loadingPlan: true, error: null });
    try {
      const plan = await invoke<CleanupPlan>('get_cleanup_plan', { sessionId });
      const initialSelected = new Set<string>();
      plan.candidates.forEach((c) => {
        if (c.selected && !c.isBlocked) {
          initialSelected.add(c.id);
        }
      });

      set({
        cleanupPlan: plan,
        selectedIds: initialSelected,
        loadingPlan: false,
      });
    } catch (err) {
      console.error('Failed to fetch cleanup plan:', err);
      set({
        error: typeof err === 'string' ? err : 'Failed to generate cleanup plan',
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
    try {
      const request: ExecuteCleanupRequest = {
        batchId: cleanupPlan.batchId,
        sessionId: cleanupPlan.sessionId,
        itemIds: Array.from(selectedIds),
        useRecycleBin,
        confirmationPhrase,
      };

      const res = await invoke<ExecuteCleanupResponse>('execute_cleanup', { request });
      set({
        executionResponse: res,
        executingCleanup: false,
        confirmationModalOpen: false,
      });

      // Refresh plan and audit history
      await fetchAuditHistory(1);
      await fetchCleanupPlan(cleanupPlan.sessionId);
      return true;
    } catch (err) {
      console.error('Cleanup execution error:', err);
      set({
        error: typeof err === 'string' ? err : 'Failed to execute cleanup operation',
        executingCleanup: false,
      });
      return false;
    }
  },

  fetchAuditHistory: async (page = 1, status) => {
    set({ auditLoading: true });
    try {
      const query: CleanupAuditQuery = {
        page,
        pageSize: 15,
        status: status || undefined,
      };
      const res = await invoke<CleanupAuditResponse>('get_cleanup_audit_history', { query });
      set({
        auditRecords: res.items,
        auditTotalItems: res.totalItems,
        auditPage: res.page,
        auditTotalPages: res.totalPages,
        auditLoading: false,
        auditStatusFilter: status || null,
      });
    } catch (err) {
      console.error('Failed to fetch audit history:', err);
      set({ auditLoading: false });
    }
  },

  resetExecutionResponse: () => set({ executionResponse: null }),
}));
