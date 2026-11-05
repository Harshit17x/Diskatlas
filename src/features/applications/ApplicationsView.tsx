import React, { useEffect, useMemo } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type { AppStatItem } from '../../types/bindings';
import { useAnalysisStore } from '../../stores/analysisStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import { Play, RotateCw, AppWindow } from 'lucide-react';

export const ApplicationsView: React.FC = () => {
  const { activeProfile, isLoading, fetchAnalysis, runClassification } = useAnalysisStore();
  const { sessionsPage, fetchSessions } = useHistoryStore();

  useEffect(() => {
    fetchSessions(1, 20);
    fetchAnalysis();
  }, [fetchSessions, fetchAnalysis]);

  // Aggregate detected software suites and packages across classification categories
  const apps: AppStatItem[] = useMemo(() => {
    if (!activeProfile?.categories) return [];
    const map = new Map<string, AppStatItem>();
    for (const cat of activeProfile.categories) {
      for (const app of cat.topApps) {
        const existing = map.get(app.appName);
        if (existing) {
          existing.fileCount += app.fileCount;
          existing.totalBytes += app.totalBytes;
        } else {
          map.set(app.appName, { ...app });
        }
      }
    }
    return Array.from(map.values()).sort((a, b) => b.totalBytes - a.totalBytes);
  }, [activeProfile]);

  const totalAppBytes = apps.reduce((acc: number, a: AppStatItem) => acc + a.totalBytes, 0);
  const totalFiles = apps.reduce((acc: number, a: AppStatItem) => acc + a.fileCount, 0);

  const handleAnalyze = () => {
    if (activeProfile?.sessionId) {
      runClassification(activeProfile.sessionId);
    } else if (sessionsPage?.items[0]?.id) {
      runClassification(sessionsPage.items[0].id);
    }
  };

  const columns: ColumnDef<AppStatItem>[] = [
    {
      accessorKey: 'appName',
      header: 'Application / Suite',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <AppWindow className="w-4 h-4 text-accent flex-shrink-0" />
          <span className="font-semibold text-primary">{row.original.appName}</span>
        </div>
      ),
    },
    {
      accessorKey: 'fileCount',
      header: 'Indexed Components',
      cell: ({ getValue }) => (
        <span className="text-secondary font-mono text-xs">{formatNumber(Number(getValue()))} files</span>
      ),
    },
    {
      accessorKey: 'totalBytes',
      header: 'Storage Footprint',
      cell: ({ getValue }) => (
        <span className="text-accent font-mono font-semibold text-xs">{formatBytes(Number(getValue()))}</span>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Applications Footprint</h1>
          <p className="text-xs text-secondary mt-0.5">
            Identify storage consumed by installed software, application caches, frameworks, and support libraries
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            size="sm"
            icon={<Play className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
            onClick={handleAnalyze}
            disabled={isLoading}
          >
            {isLoading ? 'Classifying...' : 'Analyze Applications'}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            icon={<RotateCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
            onClick={() => fetchAnalysis(activeProfile?.sessionId)}
            disabled={isLoading}
          >
            Refresh List
          </Button>
        </div>
      </div>

      {/* Info Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Audited Apps</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">{apps.length}</div>
          <span className="text-[11px] text-muted">Classified software suites</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-accent uppercase font-semibold">Total App Storage</span>
          <div className="text-xl font-bold font-mono text-accent mt-1">{formatBytes(totalAppBytes)}</div>
          <span className="text-[11px] text-muted">Binaries & application support</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Indexed App Files</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">{formatNumber(totalFiles)}</div>
          <span className="text-[11px] text-muted">Packages and dependencies</span>
        </Card>
      </div>

      {/* Applications Table */}
      <div className="space-y-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Software Registry & Packages
        </h2>
        <DataTable
          columns={columns}
          data={apps}
          emptyMessage="No application inventory has been indexed yet. Run a drive scan or click 'Analyze Applications' to map software packages and their AppData footprints."
        />
      </div>
    </div>
  );
};
