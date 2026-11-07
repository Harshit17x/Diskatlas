import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ScanFilePage,
  ScanProgress,
  ScanSummary,
} from '../types/bindings';
import { safeInvoke } from '../utils/tauri';

let pollInterval: ReturnType<typeof setInterval> | null = null;
let elapsedTimer: ReturnType<typeof setInterval> | null = null;
let scanStartTime = 0;

const stopPolling = () => {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
};

const stopElapsedTimer = () => {
  if (elapsedTimer) {
    clearInterval(elapsedTimer);
    elapsedTimer = null;
  }
};

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
    const state = get();
    set({
      isModalOpen: true,
      targetPath: targetPath ?? (state.targetPath || '/'),
      targetDriveId: targetDriveId ?? state.targetDriveId,
      summary: state.isScanning ? state.summary : null,
      error: null,
    });
  },

  closeScanModal: () => set({ isModalOpen: false }),

  setTargetPath: (path: string) => set({ targetPath: path }),
  setTargetDriveId: (driveId: string) => set({ targetDriveId: driveId }),
  setExclusions: (exclusions: string[]) => set({ exclusions }),

  startScan: async (pathOverride?: string, driveIdOverride?: string) => {
    const state = get();
    const finalPath = (pathOverride || state.targetPath || '/').trim();
    const finalDriveId = driveIdOverride || state.targetDriveId || 'DRIVE_PRIMARY';

    if (!finalPath.trim()) {
      set({ error: 'Please enter or select a valid root path to scan.' });
      return null;
    }

    // Initialize visual progress and start live elapsed timer immediately for responsive UI
    scanStartTime = Date.now();
    stopElapsedTimer();
    stopPolling();

    set({
      activeSessionId: 'starting',
      isScanning: true,
      isPaused: false,
      summary: null,
      error: null,
      targetPath: finalPath,
      targetDriveId: finalDriveId,
      progress: {
        sessionId: 'starting',
        driveId: finalDriveId,
        rootPath: finalPath,
        status: 'running',
        filesScanned: 0,
        dirsScanned: 0,
        bytesScanned: 0,
        errorCount: 0,
        currentPath: finalPath,
        currentFile: null,
        recentFiles: [],
        elapsedMillis: 0,
      },
    });

    elapsedTimer = setInterval(() => {
      const { isScanning, isPaused } = get();
      if (!isScanning) {
        stopElapsedTimer();
        return;
      }
      if (!isPaused) {
        const elapsed = Date.now() - scanStartTime;
        set((s) => (s.progress ? { progress: { ...s.progress, elapsedMillis: elapsed } } : {}));
      }
    }, 100);

    console.log('[DiskAtlas] Invoking start_scan IPC...', { finalPath, finalDriveId });
    const res = await safeInvoke<string>('start_scan', {
      path: finalPath,
      driveId: finalDriveId,
      exclusions: state.exclusions.length > 0 ? state.exclusions : null,
    });
    console.log('[DiskAtlas] start_scan response:', res);

    if (res.ok) {
      const sessionId = res.data;
      console.log('[DiskAtlas] Session started successfully:', sessionId);
      set((s) => ({
        activeSessionId: sessionId,
        progress: s.progress
          ? { ...s.progress, sessionId }
          : {
              sessionId,
              driveId: finalDriveId,
              rootPath: finalPath,
              status: 'running',
              filesScanned: 0,
              dirsScanned: 0,
              bytesScanned: 0,
              errorCount: 0,
              currentPath: finalPath,
              currentFile: null,
              recentFiles: [],
              elapsedMillis: Date.now() - scanStartTime,
            },
      }));

      // Start reliable fallback polling heartbeat (every 200ms)
      pollInterval = setInterval(async () => {
        const curSession = get().activeSessionId;
        const scanning = get().isScanning;
        if (!scanning || !curSession || curSession === 'starting') {
          return;
        }

        const pollRes = await safeInvoke<ScanProgress>('get_scan_progress', {
          sessionId: curSession,
        });

        if (pollRes.ok) {
          const p = pollRes.data;
          const isStillScanning = p.status === 'running' || p.status === 'paused';

          set({
            progress: p,
            isScanning: isStillScanning,
            isPaused: p.status === 'paused',
          });

          if (!isStillScanning) {
            stopPolling();
            stopElapsedTimer();

            // When scan completes via polling heartbeat, finalize summary and trigger analytics pipeline
            if (p.status === 'completed' || p.status === 'completedWithErrors') {
              const summary: ScanSummary = {
                sessionId: p.sessionId,
                driveId: p.driveId,
                rootPath: p.rootPath,
                status: p.status,
                totalFiles: p.filesScanned,
                totalDirs: p.dirsScanned,
                totalBytes: p.bytesScanned,
                errorCount: p.errorCount,
                durationMillis: p.elapsedMillis,
              };

              set({
                summary,
                isScanning: false,
                isPaused: false,
              });

              // Fetch files & trigger analytics update
              get().fetchScanFiles(p.sessionId, 1, 50, undefined, 'size', true);

              import('./driveStore').then(({ useDriveStore }) => {
                useDriveStore.getState().fetchDrives();
                useDriveStore.getState().fetchDbHealth();
              });
              import('./visualizationStore').then(({ useVisualizationStore }) => {
                useVisualizationStore.getState().fetchAnalytics(p.sessionId);
              });
            } else if (p.status === 'failed') {
              set({
                isScanning: false,
                isPaused: false,
                error: 'Filesystem scan encountered an unexpected failure.',
              });
            }
          }
        }
      }, 200);

      return sessionId;
    } else {
      stopElapsedTimer();
      stopPolling();
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
    stopPolling();
    stopElapsedTimer();
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    await safeInvoke('cancel_scan', { sessionId: activeSessionId });
    set({
      isScanning: false,
      isPaused: false,
    });
  },

  resetScan: () => {
    stopPolling();
    stopElapsedTimer();
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
    let unlistenError: UnlistenFn | undefined;

    try {
      unlistenProgress = await listen<ScanProgress>('scan://progress', (event) => {
        const payload = event.payload;
        set((state) => {
          // If a scan is currently active, always sync with the running scan progress
          if (
            state.activeSessionId &&
            state.activeSessionId !== 'starting' &&
            state.activeSessionId !== payload.sessionId
          ) {
            if (!state.isScanning) {
              return {};
            }
          }
          return {
            activeSessionId: payload.sessionId,
            progress: payload,
            isScanning: payload.status === 'running' || payload.status === 'paused',
            isPaused: payload.status === 'paused',
          };
        });
      });

      unlistenCompleted = await listen<ScanSummary>('scan://completed', (event) => {
        stopPolling();
        stopElapsedTimer();
        const summary = event.payload;
        set((state) => {
          if (state.activeSessionId && state.activeSessionId !== 'starting' && state.activeSessionId !== summary.sessionId) {
            if (!state.isScanning) {
              return {};
            }
          }
          return {
            activeSessionId: summary.sessionId,
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
                  currentFile: null,
                  elapsedMillis: summary.durationMillis,
                }
              : null,
          };
        });

        // Automatically fetch first page of indexed files
        get().fetchScanFiles(summary.sessionId, 1, 50, undefined, 'size', true);

        // Real-time synchronization: automatically refresh drives and storage analytics
        import('./driveStore').then(({ useDriveStore }) => {
          useDriveStore.getState().fetchDrives();
          useDriveStore.getState().fetchDbHealth();
        });
        import('./visualizationStore').then(({ useVisualizationStore }) => {
          useVisualizationStore.getState().fetchAnalytics(summary.sessionId);
        });
      });

      unlistenError = await listen<string>('scan://error', (event) => {
        stopPolling();
        stopElapsedTimer();
        set({
          isScanning: false,
          isPaused: false,
          error: event.payload || 'An unexpected scan error occurred',
        });
      });
    } catch (err) {
      console.warn('Could not register Tauri scan event listeners:', err);
    }

    return () => {
      unlistenProgress?.();
      unlistenCompleted?.();
      unlistenError?.();
    };
  },
}));

