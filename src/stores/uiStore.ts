import { create } from 'zustand';
import type { ActiveTab } from '../types/ui';

interface UIState {
  activeTab: ActiveTab;
  sidebarCollapsed: boolean;
  theme: 'dark' | 'light';
  setActiveTab: (tab: ActiveTab) => void;
  toggleSidebar: () => void;
  toggleTheme: () => void;
}

export const useUIStore = create<UIState>((set) => ({
  activeTab: 'dashboard',
  sidebarCollapsed: false,
  theme: 'dark',
  setActiveTab: (tab) => set({ activeTab: tab }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  toggleTheme: () =>
    set((s) => {
      const next = s.theme === 'dark' ? 'light' : 'dark';
      if (next === 'dark') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
      return { theme: next };
    }),
}));

