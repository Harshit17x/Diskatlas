import { create } from 'zustand';
import type { DriveInfo, SystemInfo, DatabaseHealth } from '../types/bindings';
import { safeInvoke } from '../utils/tauri';

interface DriveState {
  drives: DriveInfo[];
  selectedDriveId: string | null;
  systemInfo: SystemInfo | null;
  dbHealth: DatabaseHealth | null;
  isLoading: boolean;
  error: string | null;
  fetchDrives: () => Promise<void>;
  fetchSystemInfo: () => Promise<void>;
  fetchDbHealth: () => Promise<void>;
  selectDrive: (id: string) => void;
}

export const useDriveStore = create<DriveState>((set, get) => ({
  drives: [],
  selectedDriveId: null,
  systemInfo: null,
  dbHealth: null,
  isLoading: false,
  error: null,

  selectDrive: (id: string) => set({ selectedDriveId: id }),

  fetchDrives: async () => {
    set({ isLoading: true, error: null });
    const res = await safeInvoke<DriveInfo[]>('list_drives');
    if (res.ok) {
      set({
        drives: res.data,
        isLoading: false,
        selectedDriveId: get().selectedDriveId || res.data[0]?.id || null,
      });
    } else {
      set({ isLoading: false, error: res.error.message });
    }
  },

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

