import React from 'react';
import { Database, HardDrive, ShieldCheck, AlertCircle } from 'lucide-react';
import { useDriveStore } from '../../stores/driveStore';

export const StatusBar: React.FC = () => {
  const { drives, dbHealth, error } = useDriveStore();

  return (
    <footer className="h-7 border-t border-slate-800 bg-slate-950 px-4 flex items-center justify-between text-[11px] text-slate-400 select-none">
      <div className="flex items-center gap-4">
        {/* DB Status */}
        <div className="flex items-center gap-1.5">
          <Database className="w-3.5 h-3.5 text-sky-400" />
          <span>SQLite:</span>
          {dbHealth ? (
            <span className="flex items-center gap-1 text-emerald-400 font-medium">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
              v{dbHealth.migrationVersion} ({dbHealth.tablesCount} tables)
            </span>
          ) : (
            <span className="text-slate-500">Connecting...</span>
          )}
        </div>

        {/* Drives Count */}
        <div className="flex items-center gap-1.5">
          <HardDrive className="w-3.5 h-3.5 text-slate-400" />
          <span>Drives:</span>
          <span className="text-slate-200 font-medium">{drives.length}</span>
        </div>
      </div>

      <div className="flex items-center gap-3">
        {error ? (
          <div className="flex items-center gap-1 text-rose-400">
            <AlertCircle className="w-3.5 h-3.5" />
            <span className="truncate max-w-xs">{error}</span>
          </div>
        ) : (
          <div className="flex items-center gap-1 text-slate-400">
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
            <span>Local & Read-Only Safe</span>
          </div>
        )}
      </div>
    </footer>
  );
};

