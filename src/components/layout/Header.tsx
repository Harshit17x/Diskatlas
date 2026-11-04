import React from 'react';
import { HardDrive, RefreshCw, Sun, Moon, Search, Cpu } from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useDriveStore } from '../../stores/driveStore';
import { Badge } from '../common/Badge';

export const Header: React.FC = () => {
  const { theme, toggleTheme } = useUIStore();
  const { systemInfo, fetchDrives, fetchSystemInfo, fetchDbHealth, isLoading } = useDriveStore();

  const handleRefresh = () => {
    fetchDrives();
    fetchSystemInfo();
    fetchDbHealth();
  };

  return (
    <header className="h-14 border-b border-slate-800 bg-slate-950/80 backdrop-blur-md flex items-center justify-between px-5 select-none z-10">
      {/* Brand & System Chip */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-sky-400 to-sky-600 flex items-center justify-center shadow-lg shadow-sky-500/20">
            <HardDrive className="w-4 h-4 text-slate-950 font-bold" />
          </div>
          <div>
            <span className="font-semibold text-sm tracking-tight text-slate-100">DiskAtlas</span>
            <span className="ml-2 text-[10px] font-mono px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700">
              v{systemInfo?.appVersion || '0.1.0'}
            </span>
          </div>
        </div>

        {systemInfo && (
          <div className="hidden md:flex items-center gap-1.5 pl-3 border-l border-slate-800">
            <Badge variant="primary" size="sm">
              <Cpu className="w-3 h-3 mr-1" />
              {systemInfo.osName} ({systemInfo.arch})
            </Badge>
          </div>
        )}
      </div>

      {/* Global Quick Search Placeholder */}
      <div className="flex-1 max-w-md mx-6 hidden sm:block">
        <div className="relative">
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            placeholder="Search files, extensions, folders..."
            disabled
            className="w-full bg-slate-900 border border-slate-800 rounded-lg pl-9 pr-3 py-1.5 text-xs text-slate-400 placeholder-slate-600 focus:outline-none cursor-not-allowed"
          />
        </div>
      </div>

      {/* Action Controls */}
      <div className="flex items-center gap-2">
        <button
          onClick={handleRefresh}
          disabled={isLoading}
          title="Refresh Drives"
          className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors disabled:opacity-50"
        >
          <RefreshCw className={`w-4 h-4 ${isLoading ? 'animate-spin text-sky-400' : ''}`} />
        </button>

        <button
          onClick={toggleTheme}
          title="Toggle theme"
          className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
        >
          {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4 text-slate-800" />}
        </button>
      </div>
    </header>
  );
};

