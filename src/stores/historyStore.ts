import { create } from 'zustand';
import type {
  ScanComparisonResult,
  ScanSessionPage,
  SessionDetails,
  StorageSnapshotRecord,
} from '../types/bindings';
import { safeInvoke } from '../utils/tauri';

interface HistoryState {
  sessionsPage: ScanSessionPage | null;
  currentPage: number;
  pageSize: number;
  filterDriveId: string | null;
  filterStatus: string | null;

  selectedSessionDetails: SessionDetails | null;
  snapshots: StorageSnapshotRecord[];

  baselineSessionId: string | null;
  targetSessionId: string | null;
  comparisonResult: ScanComparisonResult | null;
  isComparing: boolean;

  isLoading: boolean;
  error: string | null;

  fetchSessions: (page?: number, pageSize?: number, driveId?: string, status?: string) => Promise<void>;
  fetchSessionDetails: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<boolean>;
  fetchSnapshots: (driveId?: string) => Promise<void>;
  createSnapshot: (driveId: string, label: string) => Promise<boolean>;
  compareScans: (baselineSessionId: string, targetSessionId: string) => Promise<void>;
  setBaselineSessionId: (id: string | null) => void;
  setTargetSessionId: (id: string | null) => void;
  clearComparison: () => void;
  clearSelectedSession: () => void;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  sessionsPage: null,
  currentPage: 1,
  pageSize: 20,
  filterDriveId: null,
  filterStatus: null,

  selectedSessionDetails: null,
  snapshots: [],

  baselineSessionId: null,
  targetSessionId: null,
  comparisonResult: null,
  isComparing: false,

  isLoading: false,
  error: null,

  fetchSessions: async (page = 1, pageSize = 20, driveId, status) => {
    set({ isLoading: true, error: null });
    const res = await safeInvoke<ScanSessionPage>('get_scan_sessions', {
      page,
      pageSize,
      driveId: driveId || get().filterDriveId || null,
      status: status || get().filterStatus || null,
    });

    if (res.ok) {
      set({
        sessionsPage: res.data,
        currentPage: page,
        pageSize,
        filterDriveId: driveId ?? get().filterDriveId,
        filterStatus: status ?? get().filterStatus,
        isLoading: false,
      });
    } else {
      set({ isLoading: false, error: res.error.message });
    }
  },

  fetchSessionDetails: async (sessionId: string) => {
    set({ isLoading: true, error: null });
    const res = await safeInvoke<SessionDetails>('get_session_details', { sessionId });
    if (res.ok) {
      set({ selectedSessionDetails: res.data, isLoading: false });
    } else {
      set({ isLoading: false, error: res.error.message });
    }
  },

  deleteSession: async (sessionId: string) => {
    const res = await safeInvoke('delete_scan_session', { sessionId });
    if (res.ok) {
      // Refresh sessions
      get().fetchSessions(get().currentPage, get().pageSize);
      if (get().selectedSessionDetails?.session.id === sessionId) {
        set({ selectedSessionDetails: null });
      }
      return true;
    }
    return false;
  },

  fetchSnapshots: async (driveId) => {
    const res = await safeInvoke<StorageSnapshotRecord[]>('get_storage_snapshots', {
      driveId: driveId || null,
      limit: 50,
    });
    if (res.ok) {
      set({ snapshots: res.data });
    }
  },

  createSnapshot: async (driveId: string, label: string) => {
    const res = await safeInvoke<StorageSnapshotRecord>('create_manual_snapshot', {
      driveId,
      label,
    });
    if (res.ok) {
      get().fetchSnapshots(driveId);
      return true;
    }
    return false;
  },

  compareScans: async (baselineSessionId: string, targetSessionId: string) => {
    set({ isComparing: true, error: null });
    const res = await safeInvoke<ScanComparisonResult>('compare_scans', {
      baselineSessionId,
      targetSessionId,
    });
    if (res.ok) {
      set({
        comparisonResult: res.data,
        baselineSessionId,
        targetSessionId,
        isComparing: false,
      });
    } else {
      set({ isComparing: false, error: res.error.message });
    }
  },

  setBaselineSessionId: (id) => set({ baselineSessionId: id }),
  setTargetSessionId: (id) => set({ targetSessionId: id }),
  clearComparison: () => set({ comparisonResult: null, baselineSessionId: null, targetSessionId: null }),
  clearSelectedSession: () => set({ selectedSessionDetails: null }),
}));
