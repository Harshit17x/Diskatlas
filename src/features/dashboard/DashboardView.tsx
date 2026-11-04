import React from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { DriveList } from '../drives/DriveList';
import { Card } from '../../components/common/Card';
import { formatBytes } from '../../utils/formatters';
import { HardDrive, Database, Server, Shield, Sparkles } from 'lucide-react';

export const DashboardView: React.FC = () => {
  const { drives, systemInfo, dbHealth } = useDriveStore();

  const totalCapacity = drives.reduce((acc, d) => acc + d.totalBytes, 0);
  const totalUsed = drives.reduce((acc, d) => acc + d.usedBytes, 0);
  const totalAvailable = drives.reduce((acc, d) => acc + d.availableBytes, 0);
  const overallPercent = totalCapacity > 0 ? (totalUsed / totalCapacity) * 100 : 0;

  return (
    <div className="space-y-6">
      {/* Overview Stat Widgets */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="p-4">
          <div className="flex items-center justify-between">
            <span className="text-xs text-slate-400 font-medium">Total Storage</span>
            <HardDrive className="w-4 h-4 text-sky-400" />
          </div>
          <div className="mt-2">
            <span className="text-2xl font-bold text-slate-100">{formatBytes(totalCapacity)}</span>
            <p className="text-[11px] text-slate-500 mt-0.5">{drives.length} volume(s) connected</p>
          </div>
        </Card>

        <Card className="p-4">
          <div className="flex items-center justify-between">
            <span className="text-xs text-slate-400 font-medium">Space Used</span>
            <div className="w-2 h-2 rounded-full bg-sky-400"></div>
          </div>
          <div className="mt-2">
            <span className="text-2xl font-bold text-slate-100">{formatBytes(totalUsed)}</span>
            <p className="text-[11px] text-sky-400 mt-0.5">{overallPercent.toFixed(1)}% total consumed</p>
          </div>
        </Card>

        <Card className="p-4">
          <div className="flex items-center justify-between">
            <span className="text-xs text-slate-400 font-medium">Free Storage</span>
            <div className="w-2 h-2 rounded-full bg-emerald-400"></div>
          </div>
          <div className="mt-2">
            <span className="text-2xl font-bold text-slate-100">{formatBytes(totalAvailable)}</span>
            <p className="text-[11px] text-emerald-400 mt-0.5">Ready for use</p>
          </div>
        </Card>

        <Card className="p-4">
          <div className="flex items-center justify-between">
            <span className="text-xs text-slate-400 font-medium">Database State</span>
            <Database className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="mt-2">
            <span className="text-2xl font-bold text-slate-100">
              {dbHealth ? `v${dbHealth.migrationVersion}` : 'Syncing'}
            </span>
            <p className="text-[11px] text-slate-500 mt-0.5">
              {dbHealth ? `${dbHealth.tablesCount} tables migrated` : 'Initializing WAL'}
            </p>
          </div>
        </Card>
      </div>

      {/* Drives Section */}
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-base font-semibold text-slate-100">Discovered Storage Drives</h2>
            <p className="text-xs text-slate-400">Hardware partitions, mounted volumes, and external drives</p>
          </div>
        </div>
        <DriveList />
      </section>

      {/* System Hardware & Architecture Specs */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* System Details */}
        <Card className="p-5 space-y-4">
          <div className="flex items-center gap-2">
            <Server className="w-4 h-4 text-sky-400" />
            <h3 className="text-sm font-semibold text-slate-100">Host Environment & Architecture</h3>
          </div>

          {systemInfo ? (
            <div className="grid grid-cols-2 gap-3 text-xs">
              <div className="p-2.5 rounded-lg bg-slate-950/60 border border-slate-800">
                <span className="text-slate-500 block">Operating System</span>
                <span className="text-slate-200 font-medium mt-0.5 block truncate">
                  {systemInfo.osName} {systemInfo.osVersion}
                </span>
              </div>
              <div className="p-2.5 rounded-lg bg-slate-950/60 border border-slate-800">
                <span className="text-slate-500 block">CPU Architecture</span>
                <span className="text-slate-200 font-medium mt-0.5 block font-mono">
                  {systemInfo.arch} ({systemInfo.cpuCount} Cores)
                </span>
              </div>
              <div className="p-2.5 rounded-lg bg-slate-950/60 border border-slate-800">
                <span className="text-slate-500 block">Host Name</span>
                <span className="text-slate-200 font-medium mt-0.5 block font-mono truncate">
                  {systemInfo.hostName}
                </span>
              </div>
              <div className="p-2.5 rounded-lg bg-slate-950/60 border border-slate-800">
                <span className="text-slate-500 block">Total RAM</span>
                <span className="text-slate-200 font-medium mt-0.5 block">
                  {formatBytes(systemInfo.totalMemoryBytes)}
                </span>
              </div>
            </div>
          ) : (
            <div className="py-4 text-center text-xs text-slate-500">Querying host specifications...</div>
          )}
        </Card>

        {/* Phase 1 Completion & Safety Guarantees */}
        <Card className="p-5 space-y-4">
          <div className="flex items-center gap-2">
            <Shield className="w-4 h-4 text-emerald-400" />
            <h3 className="text-sm font-semibold text-slate-100">Engineering & Security Policy</h3>
          </div>

          <div className="space-y-2 text-xs text-slate-400">
            <div className="flex items-start gap-2">
              <Sparkles className="w-3.5 h-3.5 text-sky-400 flex-shrink-0 mt-0.5" />
              <span>
                <strong>Zero Remote Uploads:</strong> All scanning, analysis, and indexing operations execute 100% locally.
              </span>
            </div>
            <div className="flex items-start gap-2">
              <Sparkles className="w-3.5 h-3.5 text-sky-400 flex-shrink-0 mt-0.5" />
              <span>
                <strong>Read-Only Inspection:</strong> Files are opened exclusively with non-destructive metadata permissions.
              </span>
            </div>
            <div className="flex items-start gap-2">
              <Sparkles className="w-3.5 h-3.5 text-sky-400 flex-shrink-0 mt-0.5" />
              <span>
                <strong>Symlink Safety:</strong> NTFS junction points and symlink cycles are detected and bypassed.
              </span>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
};

