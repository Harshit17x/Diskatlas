import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ScanFilePage,
  ScanProgress,
  ScanSummary,
} from '../types/bindings';
import { safeInvoke } from '../utils/tauri';

interface ScanState {
  // Modal & Target selection
  isModalOpen: boolean;
  targetDriveId: string;
  targetPath: string;
  exclusions: string[];

  // Execution state
  activeSessionId: string | null;
  isScanning: boolean;
  isPaused: boolean;
  progress: ScanProgress | null;
  summary: ScanSummary | null;
  error: string | null;

  // Paginated scanned files viewing
  filePage: ScanFilePage | null;
  currentPage: number;
  pageSize: number;
  sortBy: string;
  sortDesc: boolean;
  isLoadingFiles: boolean;

  // Actions
  openScanModal: (targetPath?: string, targetDriveId?: string) => void;
  closeScanModal: () => void;
  setTargetPath: (path: string) => void;
  setTargetDriveId: (driveId: string) => void;
  setExclusions: (exclusions: string[]) => void;

  startScan: (path?: string, driveId?: string) => Promise<string | null>;
  pauseScan: () => Promise<void>;
  resumeScan: () => Promise<void>;
  cancelScan: () => Promise<void>;
  resetScan: () => void;

  fetchScanFiles: (
    sessionId?: string,
    page?: number,
    pageSize?: number,
    parentPath?: string,
    sortBy?: string,
    sortDesc?: boolean
  ) => Promise<void>;

  setupEventListeners: () => Promise<() => void>;
}

export const useScanStore = create<ScanState>((set, get) => ({
  isModalOpen: false,
  targetDriveId: '',
  targetPath: '',
  exclusions: [],

  activeSessionId: null,
  isScanning: false,
  isPaused: false,
  progress: null,
  summary: null,
  error: null,

  filePage: null,
  currentPage: 1,
  pageSize: 50,
  sortBy: 'size',
  sortDesc: true,
  isLoadingFiles: false,

  openScanModal: (targetPath?: string, targetDriveId?: string) => {
    set({
      isModalOpen: true,
      targetPath: targetPath ?? get().targetPath,
      targetDriveId: targetDriveId ?? get().targetDriveId,
    });
  },

  closeScanModal: () => set({ isModalOpen: false }),

  setTargetPath: (path: string) => set({ targetPath: path }),
  setTargetDriveId: (driveId: string) => set({ targetDriveId: driveId }),
  setExclusions: (exclusions: string[]) => set({ exclusions }),

  startScan: async (pathOverride?: string, driveIdOverride?: string) => {
    const state = get();
    const finalPath = pathOverride || state.targetPath;
    const finalDriveId = driveIdOverride || state.targetDriveId || 'DRIVE_PRIMARY';

    if (!finalPath.trim()) {
      set({ error: 'Please enter or select a valid root path to scan.' });
      return null;
    }

    set({
      isScanning: true,
      isPaused: false,
      summary: null,
      error: null,
      targetPath: finalPath,
      targetDriveId: finalDriveId,
    });

    const res = await safeInvoke<string>('start_scan', {
      path: finalPath,
      driveId: finalDriveId,
      exclusions: state.exclusions.length > 0 ? state.exclusions : null,
    });

    if (res.ok) {
      const sessionId = res.data;
      set({
        activeSessionId: sessionId,
        progress: {
          sessionId,
          driveId: finalDriveId,
          rootPath: finalPath,
          status: 'running',
          filesScanned: 0,
          dirsScanned: 0,
          bytesScanned: 0,
          errorCount: 0,
          currentPath: finalPath,
          elapsedMillis: 0,
        },
      });
      return sessionId;
    } else {
      set({
        isScanning: false,
        error: res.error.message || 'Failed to start filesystem scan.',
      });
      return null;
    }
  },

  pauseScan: async () => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    const res = await safeInvoke('pause_scan', { sessionId: activeSessionId });
    if (res.ok) {
      set((s) => ({
        isPaused: true,
        progress: s.progress ? { ...s.progress, status: 'paused' } : null,
      }));
    }
  },

  resumeScan: async () => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    const res = await safeInvoke('resume_scan', { sessionId: activeSessionId });
    if (res.ok) {
      set((s) => ({
        isPaused: false,
        progress: s.progress ? { ...s.progress, status: 'running' } : null,
      }));
    }
  },

  cancelScan: async () => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    await safeInvoke('cancel_scan', { sessionId: activeSessionId });
    set({
      isScanning: false,
      isPaused: false,
    });
  },

  resetScan: () => {
    set({
      activeSessionId: null,
      isScanning: false,
      isPaused: false,
      progress: null,
      summary: null,
      error: null,
    });
  },

  fetchScanFiles: async (
    sessionId?: string,
    page: number = 1,
    pageSize: number = 50,
    parentPath?: string,
    sortBy: string = 'size',
    sortDesc: boolean = true
  ) => {
    const targetSessionId = sessionId || get().activeSessionId;
    if (!targetSessionId) return;

    set({ isLoadingFiles: true });
    const res = await safeInvoke<ScanFilePage>('get_scan_files', {
      sessionId: targetSessionId,
      page,
      pageSize,
      parentPath: parentPath || null,
      sortBy,
      sortDesc,
    });

    if (res.ok) {
      set({
        filePage: res.data,
        currentPage: page,
        pageSize,
        sortBy,
        sortDesc,
        isLoadingFiles: false,
      });
    } else {
      set({ isLoadingFiles: false, error: res.error.message });
    }
  },

  setupEventListeners: async () => {
    let unlistenProgress: UnlistenFn | undefined;
    let unlistenCompleted: UnlistenFn | undefined;

    try {
      unlistenProgress = await listen<ScanProgress>('scan://progress', (event) => {
        const payload = event.payload;
        set((state) => {
          // Only update if matching active session
          if (state.activeSessionId && state.activeSessionId !== payload.sessionId) {
            return {};
          }
          return {
            progress: payload,
            isScanning: payload.status === 'running' || payload.status === 'paused',
            isPaused: payload.status === 'paused',
          };
        });
      });

      unlistenCompleted = await listen<ScanSummary>('scan://completed', (event) => {
        const summary = event.payload;
        set((state) => {
          if (state.activeSessionId && state.activeSessionId !== summary.sessionId) {
            return {};
          }
          return {
            isScanning: false,
            isPaused: false,
            summary,
            progress: state.progress
              ? {
                  ...state.progress,
                  status: summary.status,
                  filesScanned: summary.totalFiles,
                  dirsScanned: summary.totalDirs,
                  bytesScanned: summary.totalBytes,
                  errorCount: summary.errorCount,
                  elapsedMillis: summary.durationMillis,
                }
              : null,
          };
        });

        // Automatically fetch first page of indexed files
        get().fetchScanFiles(summary.sessionId, 1, 50, undefined, 'size', true);
      });
    } catch (err) {
      console.warn('Could not register Tauri scan event listeners:', err);
    }

    return () => {
      unlistenProgress?.();
      unlistenCompleted?.();
    };
  },
}));

