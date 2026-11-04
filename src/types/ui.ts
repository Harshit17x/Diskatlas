export type ActiveTab = 'dashboard' | 'drives' | 'large_files' | 'duplicates' | 'recommendations' | 'settings';

export interface NavItem {
  id: ActiveTab;
  label: string;
  badge?: string | number;
}

