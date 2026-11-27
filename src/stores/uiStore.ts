import { create } from 'zustand';
import type { ActiveTab } from '../types/ui';

export type ThemeMode = 'light' | 'dark';
export type ExplorerViewMode = 'treemap' | 'sunburst' | 'analytics' | 'table';
export type CleanupTabMode = 'quick_sweep' | 'plan' | 'audit' | 'forgotten';

interface UIState {
  activeTab: ActiveTab;
  requestedExplorerView: ExplorerViewMode | null;
  requestedCleanupTab: CleanupTabMode | null;
  sidebarCollapsed: boolean;
  commandPaletteOpen: boolean;
  theme: ThemeMode;
  setActiveTab: (tab: ActiveTab) => void;
  setRequestedExplorerView: (view: ExplorerViewMode | null) => void;
  setRequestedCleanupTab: (tab: CleanupTabMode | null) => void;
  toggleSidebar: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setTheme: (theme: ThemeMode) => void;
  toggleTheme: () => void;
}

const getInitialTheme = (): ThemeMode => {
  if (typeof window !== 'undefined') {
    const saved = localStorage.getItem('diskatlas_theme') as ThemeMode | null;
    if (saved === 'dark' || saved === 'light') {
      document.documentElement.classList.toggle('dark', saved === 'dark');
      return saved;
    }
    document.documentElement.classList.remove('dark');
  }
  return 'light'; // Default to light mode as requested
};

export const useUIStore = create<UIState>((set, get) => ({
  activeTab: 'dashboard',
  requestedExplorerView: null,
  requestedCleanupTab: null,
  sidebarCollapsed: false,
  commandPaletteOpen: false,
  theme: getInitialTheme(),
  setActiveTab: (tab) => set({ activeTab: tab }),
  setRequestedExplorerView: (view) => set({ requestedExplorerView: view }),
  setRequestedCleanupTab: (tab) => set({ requestedCleanupTab: tab }),
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
