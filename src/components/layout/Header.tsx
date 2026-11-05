import React from 'react';
import {
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  Disc,
  Sun,
  Moon,
} from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useDriveStore } from '../../stores/driveStore';
import { Badge } from '../common/Badge';

export const Header: React.FC = () => {
  const { activeTab, setActiveTab, setCommandPaletteOpen, theme, toggleTheme } = useUIStore();
  const {
    drives,
    selectedDriveId,
    selectDrive,
    fetchDrives,
    fetchSystemInfo,
    fetchDbHealth,
    isLoading,
  } = useDriveStore();

  const handleRefresh = () => {
    fetchDrives();
    fetchSystemInfo();
    fetchDbHealth();
  };

  const routeTitles: Record<string, string> = {
    dashboard: 'Dashboard',
    explorer: 'Drive Explorer',
    analysis: 'Storage Analysis',
    intelligence: 'Storage Intelligence',
    developer: 'Developer Storage',
    hidden: 'Hidden Files',
    applications: 'Applications',
    duplicates: 'Duplicate Finder',
    cleanup: 'Cleanup Center',
    history: 'Storage History',
    settings: 'Settings',
  };

  const selectedDrive = drives.find((d) => d.id === selectedDriveId) || drives[0];

  return (
    <header className="h-12 border-b border-border bg-sidebar px-4 flex items-center justify-between select-none z-20">
      {/* Route Title & Volume Context */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <span className="text-xs font-semibold text-primary">
            {routeTitles[activeTab] || 'DiskAtlas'}
          </span>
          <span className="text-muted text-xs">/</span>
        </div>

        {/* Target Drive Selector */}
        {drives.length > 0 && (
          <div className="flex items-center gap-1.5">
            <Disc className="w-3.5 h-3.5 text-accent" />
            <select
              value={selectedDrive?.id || ''}
              onChange={(e) => selectDrive(e.target.value)}
              className="bg-panel border border-border rounded px-2 py-1 text-xs text-primary font-mono focus:outline-none focus:border-accent cursor-pointer"
            >
              {drives.map((d) => (
                <option key={d.id} value={d.id} className="bg-panel text-primary">
                  {d.name} {d.driveLetter ? `[${d.driveLetter}]` : `(${d.mountPoint})`}
                </option>
              ))}
            </select>
          </div>
        )}

        <Badge variant="neutral" size="sm" className="hidden sm:inline-flex">
          <ShieldCheck className="w-3 h-3 text-success mr-1" />
          Read-Only Safe
        </Badge>
      </div>

      {/* Center Search Trigger (Raycast/Linear style) */}
      <div className="flex-1 max-w-sm mx-4 hidden md:block">
        <button
          onClick={() => setCommandPaletteOpen(true)}
          className="w-full bg-panel hover:bg-elevated border border-border rounded px-3 py-1.5 text-xs text-muted flex items-center justify-between transition-colors text-left"
        >
          <div className="flex items-center gap-2">
            <Search className="w-3.5 h-3.5 text-muted" />
            <span>Search or jump to...</span>
          </div>
          <kbd className="text-[10px] font-mono px-1.5 py-0.5 rounded border border-border bg-elevated text-secondary">
            ⌘K
          </kbd>
        </button>
      </div>

      {/* Action Controls */}
      <div className="flex items-center gap-2">
        {/* Mobile Search Button */}
        <button
          onClick={() => setCommandPaletteOpen(true)}
          title="Search (⌘K)"
          className="p-1.5 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors md:hidden"
        >
          <Search className="w-4 h-4" />
        </button>

        {/* Scan Status Indicator */}
        <div className="hidden lg:flex items-center gap-2 px-2.5 py-1 rounded border border-border bg-panel text-[11px] text-secondary">
          <span className="w-2 h-2 rounded-full bg-success"></span>
          <span>Scanner Ready</span>
        </div>

        {/* Refresh Drives */}
        <button
          onClick={handleRefresh}
          disabled={isLoading}
          title="Refresh Drives"
          className="p-1.5 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors disabled:opacity-50"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin text-accent' : ''}`} />
        </button>

        {/* Light / Dark Mode Toggle */}
        <button
          onClick={toggleTheme}
          title={theme === 'light' ? 'Switch to Dark Mode' : 'Switch to Light Mode'}
          className="p-1.5 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors"
        >
          {theme === 'light' ? (
            <Moon className="w-3.5 h-3.5 text-secondary hover:text-primary" />
          ) : (
            <Sun className="w-3.5 h-3.5 text-warning" />
          )}
        </button>

        {/* Quick Settings Shortcut */}
        <button
          onClick={() => setActiveTab('settings')}
          title="Settings"
          className={`p-1.5 rounded transition-colors ${
            activeTab === 'settings'
              ? 'bg-elevated text-primary border border-border'
              : 'text-secondary hover:text-primary hover:bg-elevated'
          }`}
        >
          <Settings className="w-3.5 h-3.5" />
        </button>
      </div>
    </header>
  );
};
