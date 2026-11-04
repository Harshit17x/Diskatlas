import React from 'react';
import { Database, HardDrive, Cpu, ShieldCheck, Activity } from 'lucide-react';
import { useDriveStore } from '../../stores/driveStore';
import { formatBytes } from '../../utils/formatters';

export const StatusBar: React.FC = () => {
  const { drives, selectedDriveId, systemInfo, dbHealth, error } = useDriveStore();
  const selectedDrive = drives.find((d) => d.id === selectedDriveId) || drives[0];

  return (
    <footer className="h-6 border-t border-border bg-[#121417] px-3 flex items-center justify-between text-[11px] text-muted select-none z-20 font-mono">
      {/* Left: Storage Engine Status */}
      <div className="flex items-center gap-4">
        {/* Active Volume */}
        {selectedDrive ? (
          <div className="flex items-center gap-1.5 text-secondary">
            <HardDrive className="w-3 h-3 text-accent" />
            <span>Target:</span>
            <span className="text-primary font-medium">{selectedDrive.mountPoint}</span>
            <span className="text-muted">({formatBytes(selectedDrive.availableBytes)} free)</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-muted">
            <HardDrive className="w-3 h-3" />
            <span>No drive selected</span>
          </div>
        )}

        {/* Database WAL */}
        <div className="hidden sm:flex items-center gap-1.5">
          <Database className="w-3 h-3 text-success" />
          <span>SQLite:</span>
          {dbHealth ? (
            <span className="text-secondary">
              v{dbHealth.migrationVersion} ({dbHealth.tablesCount} tables)
            </span>
          ) : (
            <span>Connecting...</span>
          )}
        </div>

        {/* Scanner State */}
        <div className="hidden md:flex items-center gap-1.5">
          <Activity className="w-3 h-3 text-muted" />
          <span>Engine:</span>
          <span className="text-secondary">Ready (Rayon / Tokio staged)</span>
        </div>
      </div>

      {/* Right: Environment & Safety */}
      <div className="flex items-center gap-4">
        {error ? (
          <span className="text-danger truncate max-w-xs">{error}</span>
        ) : (
          <div className="hidden lg:flex items-center gap-1 text-secondary">
            <ShieldCheck className="w-3 h-3 text-success" />
            <span>Local-Only</span>
          </div>
        )}

        {systemInfo && (
          <div className="flex items-center gap-1.5 text-secondary">
            <Cpu className="w-3 h-3 text-accent" />
            <span>{systemInfo.arch}</span>
            <span className="text-muted">({systemInfo.cpuCount}c)</span>
          </div>
        )}
      </div>
    </footer>
  );
};
