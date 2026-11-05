import { create } from 'zustand';
import type { ActiveTab } from '../types/ui';

export type ThemeMode = 'light' | 'dark';

interface UIState {
  activeTab: ActiveTab;
  sidebarCollapsed: boolean;
  commandPaletteOpen: boolean;
  theme: ThemeMode;
  setActiveTab: (tab: ActiveTab) => void;
  toggleSidebar: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setTheme: (theme: ThemeMode) => void;
  toggleTheme: () => void;
}

const getInitialTheme = (): ThemeMode => {
  if (typeof window !== 'undefined') {
    const saved = localStorage.getItem('diskatlas_theme') as ThemeMode | null;
    if (saved === 'dark' || saved === 'light') return saved;
  }
  return 'light'; // Default to light mode as requested
};

export const useUIStore = create<UIState>((set, get) => ({
  activeTab: 'dashboard',
  sidebarCollapsed: false,
  commandPaletteOpen: false,
  theme: getInitialTheme(),
  setActiveTab: (tab) => set({ activeTab: tab }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
  setTheme: (theme) => {
    localStorage.setItem('diskatlas_theme', theme);
    set({ theme });
  },
  toggleTheme: () => {
    const nextTheme: ThemeMode = get().theme === 'light' ? 'dark' : 'light';
    localStorage.setItem('diskatlas_theme', nextTheme);
    set({ theme: nextTheme });
  },
}));
