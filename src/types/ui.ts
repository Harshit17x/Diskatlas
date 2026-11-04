export type ActiveTab =
  | 'dashboard'
  | 'explorer'
  | 'analysis'
  | 'intelligence'
  | 'developer'
  | 'hidden'
  | 'applications'
  | 'duplicates'
  | 'cleanup'
  | 'history'
  | 'settings';

export interface NavItem {
  id: ActiveTab;
  label: string;
  badge?: string | number;
  shortcut?: string;
  category: 'core' | 'intelligence' | 'system';
}

export type ScanStatus = 'idle' | 'ready' | 'scanning' | 'completed' | 'paused';
