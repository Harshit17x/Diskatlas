import React from 'react';
import {
  LayoutDashboard,
  HardDrive,
  FileQuestion,
  Copy,
  Sparkles,
  Settings,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import type { ActiveTab } from '../../types/ui';

interface NavButtonProps {
  id: ActiveTab;
  icon: React.ReactNode;
  label: string;
  badge?: string;
  active: boolean;
  collapsed: boolean;
  onClick: () => void;
}

const NavButton: React.FC<NavButtonProps> = ({
  icon,
  label,
  badge,
  active,
  collapsed,
  onClick,
}) => {
  return (
    <button
      onClick={onClick}
      title={collapsed ? label : undefined}
      className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-xs font-medium transition-all ${
        active
          ? 'bg-sky-500/10 text-sky-400 font-semibold'
          : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900'
      }`}
    >
      <div className={`flex-shrink-0 ${active ? 'text-sky-400' : 'text-slate-400'}`}>{icon}</div>
      {!collapsed && (
        <span className="flex-1 text-left truncate">{label}</span>
      )}
      {!collapsed && badge && (
        <span className="text-[10px] font-mono px-1.5 py-0.5 rounded-full bg-slate-800 text-slate-400 border border-slate-700">
          {badge}
        </span>
      )}
    </button>
  );
};

export const Sidebar: React.FC = () => {
  const { activeTab, setActiveTab, sidebarCollapsed, toggleSidebar } = useUIStore();

  const navItems: Array<{ id: ActiveTab; label: string; icon: React.ReactNode; badge?: string }> = [
    { id: 'dashboard', label: 'Dashboard', icon: <LayoutDashboard className="w-4 h-4" /> },
    { id: 'drives', label: 'Storage Drives', icon: <HardDrive className="w-4 h-4" /> },
    { id: 'large_files', label: 'Large Files', icon: <FileQuestion className="w-4 h-4" />, badge: 'Ph 3' },
    { id: 'duplicates', label: 'Duplicates', icon: <Copy className="w-4 h-4" />, badge: 'Ph 4' },
    { id: 'recommendations', label: 'Optimizations', icon: <Sparkles className="w-4 h-4" />, badge: 'Ph 5' },
    { id: 'settings', label: 'Settings', icon: <Settings className="w-4 h-4" /> },
  ];

  return (
    <aside
      className={`border-r border-slate-800 bg-slate-950 flex flex-col justify-between transition-all duration-200 select-none ${
        sidebarCollapsed ? 'w-14' : 'w-56'
      }`}
    >
      <div className="p-3 space-y-1.5">
        <div className="px-2 py-1.5 text-[10px] font-semibold text-slate-500 uppercase tracking-wider">
          {!sidebarCollapsed ? 'Navigation' : '•••'}
        </div>
        {navItems.map((item) => (
          <NavButton
            key={item.id}
            id={item.id}
            icon={item.icon}
            label={item.label}
            badge={item.badge}
            active={activeTab === item.id}
            collapsed={sidebarCollapsed}
            onClick={() => setActiveTab(item.id)}
          />
        ))}
      </div>

      <div className="p-3 border-t border-slate-800/80">
        <button
          onClick={toggleSidebar}
          className="w-full flex items-center justify-center p-2 rounded-lg text-slate-500 hover:text-slate-300 hover:bg-slate-900 transition-colors"
          title={sidebarCollapsed ? 'Expand Sidebar' : 'Collapse Sidebar'}
        >
          {sidebarCollapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
        </button>
      </div>
    </aside>
  );
};

