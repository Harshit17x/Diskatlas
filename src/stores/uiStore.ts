import { create } from 'zustand';
import type { ActiveTab } from '../types/ui';

interface UIState {
  activeTab: ActiveTab;
  sidebarCollapsed: boolean;
  commandPaletteOpen: boolean;
  setActiveTab: (tab: ActiveTab) => void;
  toggleSidebar: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
}

export const useUIStore = create<UIState>((set) => ({
  activeTab: 'dashboard',
  sidebarCollapsed: false,
  commandPaletteOpen: false,
  setActiveTab: (tab) => set({ activeTab: tab }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
}));
