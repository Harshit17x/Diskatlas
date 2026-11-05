import React from 'react';
import {
  LayoutDashboard,
  HardDrive,
  PieChart,
  Brain,
  Terminal,
  EyeOff,
  Box,
  Copy,
  Sparkles,
  History,
  Settings,
  ChevronLeft,
  ChevronRight,
  Database,
} from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useDriveStore } from '../../stores/driveStore';
import type { ActiveTab, NavItem } from '../../types/ui';

export const Sidebar: React.FC = () => {
  const { activeTab, setActiveTab, sidebarCollapsed, toggleSidebar } = useUIStore();
  const { dbHealth } = useDriveStore();

  const navItems: NavItem[] = [
    // Core
    { id: 'dashboard', label: 'Dashboard', category: 'core', shortcut: '⌘1' },
    { id: 'explorer', label: 'Drive Explorer', category: 'core', shortcut: '⌘2' },
    { id: 'analysis', label: 'Storage Analysis', category: 'core', shortcut: '⌘3' },

    // Intelligence
    { id: 'intelligence', label: 'Storage Intelligence', category: 'intelligence' },
    { id: 'developer', label: 'Developer Storage', category: 'intelligence' },
    { id: 'hidden', label: 'Hidden Files', category: 'intelligence' },
    { id: 'applications', label: 'Applications', category: 'intelligence' },
    { id: 'duplicates', label: 'Duplicate Finder', category: 'intelligence' },
    { id: 'cleanup', label: 'Cleanup Center', category: 'intelligence' },

    // System
    { id: 'history', label: 'Storage History', category: 'system' },
    { id: 'settings', label: 'Settings', category: 'system' },
  ];

  const getIcon = (id: ActiveTab) => {
    switch (id) {
      case 'dashboard':
        return <LayoutDashboard className="w-4 h-4" />;
      case 'explorer':
        return <HardDrive className="w-4 h-4" />;
      case 'analysis':
        return <PieChart className="w-4 h-4" />;
      case 'intelligence':
        return <Brain className="w-4 h-4" />;
      case 'developer':
        return <Terminal className="w-4 h-4" />;
      case 'hidden':
        return <EyeOff className="w-4 h-4" />;
      case 'applications':
        return <Box className="w-4 h-4" />;
      case 'duplicates':
        return <Copy className="w-4 h-4" />;
      case 'cleanup':
        return <Sparkles className="w-4 h-4" />;
      case 'history':
        return <History className="w-4 h-4" />;
      case 'settings':
        return <Settings className="w-4 h-4" />;
    }
  };

  const renderNavGroup = (category: 'core' | 'intelligence' | 'system', title: string) => {
    const items = navItems.filter((item) => item.category === category);
    return (
      <div className="space-y-0.5">
        {!sidebarCollapsed && (
          <div className="px-3 pt-3 pb-1 text-[10px] font-semibold tracking-wider uppercase text-muted">
            {title}
          </div>
        )}
        {items.map((item) => {
          const isActive = activeTab === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setActiveTab(item.id)}
              title={sidebarCollapsed ? item.label : undefined}
              className={`w-full flex items-center gap-2.5 px-3 py-1.5 text-xs font-medium rounded transition-colors ${
                isActive
                  ? 'bg-elevated text-primary font-semibold border-l-2 border-accent'
                  : 'text-secondary hover:text-primary hover:bg-elevated border-l-2 border-transparent'
              }`}
            >
              <span className={`flex-shrink-0 ${isActive ? 'text-accent' : 'text-muted'}`}>
                {getIcon(item.id)}
              </span>
              {!sidebarCollapsed && (
                <span className="flex-1 text-left truncate">{item.label}</span>
              )}
              {!sidebarCollapsed && item.badge && (
                <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-panel text-muted border border-border">
                  {item.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>
    );
  };

  return (
    <aside
      className={`border-r border-border bg-sidebar flex flex-col justify-between transition-all duration-150 select-none z-10 ${
        sidebarCollapsed ? 'w-14' : 'w-60'
      }`}
    >
      {/* Top Application Header */}
      <div className="flex flex-col flex-1 overflow-hidden">
        <div className="h-12 border-b border-border flex items-center px-3.5 gap-2.5 bg-sidebar">
          <div className="w-6 h-6 rounded bg-accent flex items-center justify-center text-white flex-shrink-0 shadow-sm">
            <HardDrive className="w-3.5 h-3.5 font-bold" />
          </div>
          {!sidebarCollapsed && (
            <div className="flex-1 min-w-0">
              <span className="text-xs font-bold tracking-tight text-primary truncate block">
                DiskAtlas
              </span>
              <span className="text-[10px] text-muted font-mono block -mt-0.5">
                Storage Intelligence
              </span>
            </div>
          )}
        </div>

        {/* Navigation Items (Scrollable) */}
        <div className="flex-1 overflow-y-auto px-2 py-2 space-y-2">
          {renderNavGroup('core', 'Core')}
          {renderNavGroup('intelligence', 'Storage Intelligence')}
          {renderNavGroup('system', 'System & Preferences')}
        </div>
      </div>

      {/* Bottom Status / Collapse Bar */}
      <div className="p-2 border-t border-border bg-sidebar space-y-1">
        {!sidebarCollapsed && dbHealth && (
          <div className="px-2 py-1 flex items-center justify-between text-[10px] text-muted font-mono">
            <div className="flex items-center gap-1.5 truncate">
              <Database className="w-3 h-3 text-success flex-shrink-0" />
              <span className="truncate">SQLite v{dbHealth.migrationVersion}</span>
            </div>
            <span className="text-success">WAL</span>
          </div>
        )}

        <button
          onClick={toggleSidebar}
          className="w-full flex items-center justify-center p-1.5 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors"
          title={sidebarCollapsed ? 'Expand Sidebar' : 'Collapse Sidebar'}
        >
          {sidebarCollapsed ? (
            <ChevronRight className="w-3.5 h-3.5" />
          ) : (
            <div className="flex items-center gap-2 text-xs">
              <ChevronLeft className="w-3.5 h-3.5" />
              <span>Collapse Sidebar</span>
            </div>
          )}
        </button>
      </div>
    </aside>
  );
};
