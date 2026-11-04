import { create } from 'zustand';
import type { DriveInfo, DriveDetails, SystemInfo, DatabaseHealth } from '../types/bindings';
import { safeInvoke } from '../utils/tauri';

interface DriveState {
  drives: DriveInfo[];
  selectedDriveId: string | null;
  activeDriveDetails: DriveDetails | null;
  isDetailModalOpen: boolean;
  systemInfo: SystemInfo | null;
  dbHealth: DatabaseHealth | null;
  isLoading: boolean;
  isInspecting: boolean;
  error: string | null;

  fetchDrives: () => Promise<void>;
  fetchDriveDetails: (id: string) => Promise<DriveDetails | null>;
  openDetailModal: (id: string) => Promise<void>;
  closeDetailModal: () => void;
  fetchSystemInfo: () => Promise<void>;
  fetchDbHealth: () => Promise<void>;
  selectDrive: (id: string) => void;
}

export const useDriveStore = create<DriveState>((set, get) => ({
  drives: [],
  selectedDriveId: null,
  activeDriveDetails: null,
  isDetailModalOpen: false,
  systemInfo: null,
  dbHealth: null,
  isLoading: false,
  isInspecting: false,
  error: null,

  selectDrive: (id: string) => set({ selectedDriveId: id }),

  fetchDrives: async () => {
    set({ isLoading: true, error: null });
    // First try the new typed Phase 3 command get_drives, fallback to list_drives
    const res = await safeInvoke<DriveInfo[]>('get_drives');
    if (res.ok) {
      set({
        drives: res.data,
        isLoading: false,
        selectedDriveId: get().selectedDriveId || res.data[0]?.id || null,
      });
    } else {
      // Fallback
      const fallbackRes = await safeInvoke<DriveInfo[]>('list_drives');
      if (fallbackRes.ok) {
        set({
          drives: fallbackRes.data,
          isLoading: false,
          selectedDriveId: get().selectedDriveId || fallbackRes.data[0]?.id || null,
        });
      } else {
        set({ isLoading: false, error: res.error.message });
      }
    }
  },

  fetchDriveDetails: async (id: string) => {
    set({ isInspecting: true });
    const res = await safeInvoke<DriveDetails>('get_drive_details', { driveId: id });
    set({ isInspecting: false });
    if (res.ok) {
      return res.data;
    }
    return null;
  },

  openDetailModal: async (id: string) => {
    set({ isDetailModalOpen: true, isInspecting: true });
    const res = await safeInvoke<DriveDetails>('get_drive_details', { driveId: id });
    set({
      isInspecting: false,
      activeDriveDetails: res.ok ? res.data : null,
    });
  },

  closeDetailModal: () => set({ isDetailModalOpen: false, activeDriveDetails: null }),

  fetchSystemInfo: async () => {
    const res = await safeInvoke<SystemInfo>('get_system_info');
    if (res.ok) {
      set({ systemInfo: res.data });
    }
  },

  fetchDbHealth: async () => {
    const res = await safeInvoke<DatabaseHealth>('get_database_health');
    if (res.ok) {
      set({ dbHealth: res.data });
    }
  },
}));
