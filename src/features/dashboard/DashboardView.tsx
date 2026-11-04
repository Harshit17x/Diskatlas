import React from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { useUIStore } from '../../stores/uiStore';
import { DriveList } from '../drives/DriveList';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { formatBytes } from '../../utils/formatters';
import {
  HardDrive,
  Database,
  Server,
  Shield,
  ArrowRight,
  PieChart,
  Terminal,
} from 'lucide-react';

export const DashboardView: React.FC = () => {
  const { drives, systemInfo, dbHealth } = useDriveStore();
  const { setActiveTab } = useUIStore();

  const totalCapacity = drives.reduce((acc, d) => acc + (d.totalBytes ?? 0), 0);
  const totalUsed = drives.reduce((acc, d) => acc + (d.usedBytes ?? 0), 0);
  const totalAvailable = drives.reduce((acc, d) => acc + (d.availableBytes ?? 0), 0);
  const overallPercent = totalCapacity > 0 ? (totalUsed / totalCapacity) * 100 : 0;

  return (
    <div className="space-y-5">
      {/* Top Banner / Actions */}
      <div className="flex items-center justify-between pb-2 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Storage Overview</h1>
          <p className="text-xs text-secondary mt-0.5">
            Physical hardware partitions, storage utilization, and system diagnostics
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            icon={<PieChart className="w-3.5 h-3.5" />}
            onClick={() => setActiveTab('analysis')}
          >
            Analyze Categories
          </Button>
          <Button
            variant="primary"
            size="sm"
            icon={<ArrowRight className="w-3.5 h-3.5" />}
            onClick={() => setActiveTab('explorer')}
          >
            Drive Explorer
          </Button>
        </div>
      </div>

      {/* Overview Stat Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Total Storage</span>
            <HardDrive className="w-3.5 h-3.5 text-accent" />
          </div>
          <div className="mt-2">
            <div className="text-xl font-bold font-mono text-primary">{formatBytes(totalCapacity)}</div>
            <p className="text-[11px] text-muted font-mono mt-0.5">{drives.length} volume(s) connected</p>
          </div>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Used Space</span>
            <span className="w-2 h-2 rounded-full bg-accent"></span>
          </div>
          <div className="mt-2">
            <div className="text-xl font-bold font-mono text-primary">{formatBytes(totalUsed)}</div>
            <p className="text-[11px] text-accent font-mono mt-0.5">{overallPercent.toFixed(1)}% consumed</p>
          </div>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Available Free</span>
            <span className="w-2 h-2 rounded-full bg-success"></span>
          </div>
          <div className="mt-2">
            <div className="text-xl font-bold font-mono text-primary">{formatBytes(totalAvailable)}</div>
            <p className="text-[11px] text-success font-mono mt-0.5">Unallocated storage</p>
          </div>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Database Engine</span>
            <Database className="w-3.5 h-3.5 text-success" />
          </div>
          <div className="mt-2">
            <div className="text-xl font-bold font-mono text-primary">
              {dbHealth ? `v${dbHealth.migrationVersion}` : 'Syncing'}
            </div>
            <p className="text-[11px] text-muted font-mono mt-0.5">
              {dbHealth ? `${dbHealth.tablesCount} schema tables active` : 'Initializing WAL'}
            </p>
          </div>
        </Card>
      </div>

      {/* Discovered Storage Drives */}
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
            Connected Storage Volumes
          </h2>
        </div>
        <DriveList />
      </section>

      {/* Hardware Specifications & Security Guarantees */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {/* Host Diagnostics */}
        <Card padding="md" className="space-y-3">
          <div className="flex items-center gap-2 pb-2 border-b border-border">
            <Server className="w-4 h-4 text-accent" />
            <h3 className="text-xs font-semibold text-primary">Host Environment & Diagnostics</h3>
          </div>

          {systemInfo ? (
            <div className="grid grid-cols-2 gap-2 text-xs font-mono">
              <div className="p-2.5 rounded bg-[#15171B] border border-border">
                <span className="text-muted block text-[10px] uppercase font-sans">Operating System</span>
                <span className="text-primary font-medium mt-0.5 block truncate">
                  {systemInfo.osName} {systemInfo.osVersion}
                </span>
              </div>
              <div className="p-2.5 rounded bg-[#15171B] border border-border">
                <span className="text-muted block text-[10px] uppercase font-sans">CPU Architecture</span>
                <span className="text-primary font-medium mt-0.5 block">
                  {systemInfo.arch} ({systemInfo.cpuCount} Cores)
                </span>
              </div>
              <div className="p-2.5 rounded bg-[#15171B] border border-border">
                <span className="text-muted block text-[10px] uppercase font-sans">Host Identifier</span>
                <span className="text-primary font-medium mt-0.5 block truncate">
                  {systemInfo.hostName}
                </span>
              </div>
              <div className="p-2.5 rounded bg-[#15171B] border border-border">
                <span className="text-muted block text-[10px] uppercase font-sans">System Memory</span>
                <span className="text-primary font-medium mt-0.5 block">
                  {formatBytes(systemInfo.totalMemoryBytes)} RAM
                </span>
              </div>
            </div>
          ) : (
            <div className="py-4 text-center text-xs text-muted">Querying host specifications...</div>
          )}
        </Card>

        {/* Intelligence Quick Jump */}
        <Card padding="md" className="space-y-3">
          <div className="flex items-center gap-2 pb-2 border-b border-border">
            <Shield className="w-4 h-4 text-success" />
            <h3 className="text-xs font-semibold text-primary">Intelligence Modules</h3>
          </div>

          <div className="grid grid-cols-2 gap-2">
            <button
              onClick={() => setActiveTab('developer')}
              className="p-3 text-left rounded bg-[#15171B] border border-border hover:bg-elevated hover:border-border-light transition-colors group"
            >
              <Terminal className="w-4 h-4 text-accent mb-1 group-hover:translate-x-0.5 transition-transform" />
              <div className="text-xs font-semibold text-primary">Developer Storage</div>
              <p className="text-[11px] text-muted mt-0.5">node_modules, target, Docker caches</p>
            </button>

            <button
              onClick={() => setActiveTab('duplicates')}
              className="p-3 text-left rounded bg-[#15171B] border border-border hover:bg-elevated hover:border-border-light transition-colors group"
            >
              <HardDrive className="w-4 h-4 text-warning mb-1 group-hover:translate-x-0.5 transition-transform" />
              <div className="text-xs font-semibold text-primary">Duplicate Finder</div>
              <p className="text-[11px] text-muted mt-0.5">Identify redundant multi-gigabyte files</p>
            </button>
          </div>
        </Card>
      </div>
    </div>
  );
};
