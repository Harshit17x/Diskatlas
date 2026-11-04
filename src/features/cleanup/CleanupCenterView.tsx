import React, { useEffect, useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type { ForgottenFileItem } from '../../types/bindings';
import { useAdvancedAnalysisStore } from '../../stores/advancedAnalysisStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  ShieldCheck,
  CheckCircle2,
  AlertTriangle,
  RotateCw,
  Clock,
  Archive,
  Download,
  Terminal,
  FileCode,
  Package,
  ChevronLeft,
  ChevronRight,
  Info,
} from 'lucide-react';

interface Recommendation {
  id: string;
  title: string;
  category: 'Developer' | 'System' | 'Applications';
  safetyLevel: 'Safe' | 'Review Required';
  estimatedSavings: string;
  reason: string;
}

export const CleanupCenterView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('forgotten');

  const {
    forgottenFiles,
    forgottenFilesLoading,
    forgottenFilesQuery,
    setForgottenFilesQuery,
    fetchForgottenFiles,
    selectedSessionId,
    setSelectedSessionId,
  } = useAdvancedAnalysisStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();

  useEffect(() => {
    fetchSessions(1, 20);
    fetchForgottenFiles();
  }, [fetchSessions, fetchForgottenFiles]);

  const handleSessionChange = (sid: string) => {
    setSelectedSessionId(sid || null);
    fetchForgottenFiles(sid || undefined);
  };

  const handleAgeChange = (days: number) => {
    setForgottenFilesQuery({ minAgeDays: days, page: 1 });
    fetchForgottenFiles();
  };

  const handleSignalChange = (signal: string) => {
    setForgottenFilesQuery({
      signalType: signal === 'all' ? undefined : signal,
      page: 1,
    });
    fetchForgottenFiles();
  };

  const getSignalBadge = (signal: string) => {
    switch (signal) {
      case 'old_download':
        return (
          <Badge variant="accent" size="sm" icon={<Download className="w-3 h-3" />}>
            Old Download
          </Badge>
        );
      case 'old_installer':
        return (
          <Badge variant="warning" size="sm" icon={<Package className="w-3 h-3" />}>
            Old Installer
          </Badge>
        );
      case 'old_archive':
        return (
          <Badge variant="neutral" size="sm" icon={<Archive className="w-3 h-3" />}>
            Old Archive
          </Badge>
        );
      case 'old_backup':
        return (
          <Badge variant="neutral" size="sm" icon={<FileCode className="w-3 h-3" />}>
            Old Backup
          </Badge>
        );
      case 'stale_build_artifact':
        return (
          <Badge variant="warning" size="sm" icon={<Terminal className="w-3 h-3" />}>
            Stale Build Output
          </Badge>
        );
      default:
        return <Badge variant="neutral" size="sm">{signal}</Badge>;
    }
  };

  const forgottenColumns: ColumnDef<ForgottenFileItem>[] = [
    {
      accessorKey: 'name',
      header: 'File Name',
      cell: ({ row }) => (
        <span className="font-sans font-medium text-primary truncate block max-w-xs" title={row.original.path}>
          {row.original.name}
        </span>
      ),
    },
    {
      accessorKey: 'signalType',
      header: 'Heuristic Signal',
      cell: ({ row }) => getSignalBadge(row.original.signalType),
    },
    {
      accessorKey: 'ageDays',
      header: 'Untouched Age',
      cell: ({ getValue }) => (
        <div className="flex items-center gap-1.5 text-secondary font-mono text-xs">
          <Clock className="w-3.5 h-3.5 text-muted shrink-0" />
          <span>{Number(getValue())} days</span>
        </div>
      ),
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Size',
      cell: ({ getValue }) => (
        <span className="font-mono text-accent font-semibold text-xs">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'path',
      header: 'Location & Advisory Note',
      cell: ({ row }) => (
        <div className="space-y-0.5 max-w-md">
          <span className="font-mono text-secondary text-[11px] truncate block" title={row.original.path}>
            {row.original.path}
          </span>
          <span className="text-[10px] text-muted block truncate italic" title={row.original.safetyHeuristic}>
            {row.original.safetyHeuristic}
          </span>
        </div>
      ),
    },
  ];

  const recommendations: Recommendation[] = [
    {
      id: 'rec-1',
      title: 'Stale node_modules (> 60 days unaccessed)',
      category: 'Developer',
      safetyLevel: 'Safe',
      estimatedSavings: 'Review in Developer Storage tab',
      reason: 'Can be reinstalled at any time via npm install / pnpm install',
    },
    {
      id: 'rec-2',
      title: 'Rust / Cargo incremental target directories',
      category: 'Developer',
      safetyLevel: 'Safe',
      estimatedSavings: 'Review in Developer Storage tab',
      reason: 'Recompilable build caches',
    },
    {
      id: 'rec-3',
      title: 'System temporary log dumps & crash reports',
      category: 'System',
      safetyLevel: 'Safe',
      estimatedSavings: 'Review in Hidden & System tab',
      reason: 'Expired diagnostic traces older than 14 days',
    },
    {
      id: 'rec-4',
      title: 'User Trash / Recycle Bin contents',
      category: 'System',
      safetyLevel: 'Review Required',
      estimatedSavings: 'Review in Explorer tab',
      reason: 'User deleted items waiting for permanent purge',
    },
  ];

  const recColumns: ColumnDef<Recommendation>[] = [
    {
      accessorKey: 'title',
      header: 'Optimization Recommendation',
      cell: ({ row }) => (
        <div>
          <span className="font-semibold text-primary">{row.original.title}</span>
          <p className="text-[11px] text-secondary mt-0.5">{row.original.reason}</p>
        </div>
      ),
    },
    {
      accessorKey: 'category',
      header: 'Category',
      cell: ({ getValue }) => <Badge variant="neutral" size="sm">{String(getValue())}</Badge>,
    },
    {
      accessorKey: 'safetyLevel',
      header: 'Safety Level',
      cell: ({ getValue }) => (
        <Badge
          variant={getValue() === 'Safe' ? 'success' : 'warning'}
          size="sm"
          icon={getValue() === 'Safe' ? <CheckCircle2 className="w-3 h-3 text-success" /> : <AlertTriangle className="w-3 h-3 text-warning" />}
        >
          {String(getValue())}
        </Badge>
      ),
    },
    {
      accessorKey: 'estimatedSavings',
      header: 'Audit Location',
      cell: ({ getValue }) => <span className="text-secondary text-xs">{String(getValue())}</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* 1. Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Storage Optimization & Forgotten Files</h1>
          <p className="text-xs text-secondary mt-0.5">
            Identify old downloads, standalone installers, stale archives, and forgotten project build trees
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Target Session Selector */}
          <select
            value={selectedSessionId || ''}
            onChange={(e) => handleSessionChange(e.target.value)}
            className="bg-[#15171B] border border-border rounded px-2.5 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          >
            <option value="">Latest Completed Scan</option>
            {sessionsPage?.items.map((s) => (
              <option key={s.id} value={s.id}>
                Session {s.id.slice(0, 8)} ({s.driveId} • {formatBytes(s.totalBytes)})
              </option>
            ))}
          </select>

          <Button
            variant="secondary"
            size="sm"
            icon={<RotateCw className={`w-3.5 h-3.5 ${forgottenFilesLoading ? 'animate-spin' : ''}`} />}
            onClick={() => fetchForgottenFiles()}
            disabled={forgottenFilesLoading}
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* 2. Safety Philosophy Notice */}
      <div className="p-3 bg-[#16181C] border border-border rounded flex items-start gap-2.5 text-xs text-secondary">
        <Info className="w-4 h-4 text-accent shrink-0 mt-0.5" />
        <div className="space-y-0.5 text-[11px] leading-relaxed">
          <span className="font-semibold text-primary block">Crucial Heuristic Principle:</span>
          <p>
            Age is only a signal, <strong className="text-primary font-semibold">not proof</strong> that a file is unnecessary.
            DiskAtlas operates strictly in non-destructive, read-only mode during analysis. No automated deletions or silent sweeps will ever be performed.
          </p>
        </div>
      </div>

      {/* 3. Tabs */}
      <Tabs
        tabs={[
          { id: 'forgotten', label: `Forgotten Files (${formatNumber(forgottenFiles?.totalItems ?? 0)})` },
          { id: 'blueprints', label: 'Optimization Blueprints' },
        ]}
        activeTab={activeTab}
        onChange={setActiveTab}
      />

      {/* Tab 1: Forgotten Files */}
      {activeTab === 'forgotten' && (
        <div className="space-y-4">
          {/* Metrics */}
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
            <Card padding="md">
              <span className="text-[11px] text-muted uppercase font-semibold">Stale Storage</span>
              <div className="text-xl font-bold font-mono text-warning mt-1">
                {formatBytes(forgottenFiles?.totalBytes ?? 0)}
              </div>
              <span className="text-[11px] text-muted font-mono">{formatNumber(forgottenFiles?.totalItems ?? 0)} items identified</span>
            </Card>

            <Card padding="md">
              <span className="text-[11px] text-muted uppercase font-semibold">Age Threshold</span>
              <div className="text-xl font-bold font-mono text-primary mt-1">
                ≥ {forgottenFilesQuery.minAgeDays ?? 90} Days
              </div>
              <span className="text-[11px] text-muted">Unmodified since cutoff</span>
            </Card>

            <Card padding="md">
              <span className="text-[11px] text-muted uppercase font-semibold">Audit Status</span>
              <div className="text-xs font-semibold text-success mt-2">Advisory Signals Only</div>
              <span className="text-[11px] text-muted">Requires manual review</span>
            </Card>

            <Card padding="md">
              <span className="text-[11px] text-muted uppercase font-semibold">Safety Level</span>
              <div className="text-xs font-semibold text-primary mt-2">Zero Blind Purges</div>
              <span className="text-[11px] text-muted">Read-only intelligence</span>
            </Card>
          </div>

          {/* Filter Bar */}
          <Card padding="md" className="space-y-2">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
              <div className="flex items-center gap-1.5 flex-wrap">
                <span className="text-muted font-medium mr-1">Age Filter:</span>
                {[
                  { label: '≥ 30 Days', days: 30 },
                  { label: '≥ 60 Days', days: 60 },
                  { label: '≥ 90 Days', days: 90 },
                  { label: '≥ 180 Days', days: 180 },
                  { label: '≥ 1 Year', days: 365 },
                ].map((a) => (
                  <button
                    key={a.label}
                    onClick={() => handleAgeChange(a.days)}
                    className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors ${
                      forgottenFilesQuery.minAgeDays === a.days
                        ? 'bg-accent text-white font-semibold'
                        : 'bg-elevated text-secondary hover:text-primary hover:bg-[#282D35]'
                    }`}
                  >
                    {a.label}
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-1.5 flex-wrap">
                <span className="text-muted font-medium mr-1">Signal:</span>
                {[
                  { id: 'all', label: 'All' },
                  { id: 'old_download', label: 'Downloads' },
                  { id: 'old_installer', label: 'Installers' },
                  { id: 'old_archive', label: 'Archives' },
                  { id: 'old_backup', label: 'Backups' },
                  { id: 'stale_build_artifact', label: 'Builds' },
                ].map((s) => (
                  <button
                    key={s.id}
                    onClick={() => handleSignalChange(s.id)}
                    className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                      (forgottenFilesQuery.signalType || 'all') === s.id
                        ? 'bg-accent text-white font-medium'
                        : 'bg-elevated text-secondary hover:text-primary hover:bg-[#282D35]'
                    }`}
                  >
                    {s.label}
                  </button>
                ))}
              </div>
            </div>
          </Card>

          {/* Data Table */}
          <div className="space-y-2">
            <DataTable
              columns={forgottenColumns}
              data={forgottenFiles?.items || []}
              emptyMessage={
                forgottenFilesLoading
                  ? 'Analyzing file age signals...'
                  : 'No files matched the selected age and signal criteria.'
              }
            />

            {/* Pagination */}
            {forgottenFiles && forgottenFiles.totalPages > 1 && (
              <div className="flex items-center justify-between px-2 pt-2 text-xs text-secondary border-t border-border">
                <span>
                  Page {forgottenFiles.page} of {forgottenFiles.totalPages} ({formatNumber(forgottenFiles.totalItems)} total items)
                </span>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    icon={<ChevronLeft className="w-3.5 h-3.5" />}
                    onClick={() => {
                      setForgottenFilesQuery({ page: forgottenFiles.page - 1 });
                      fetchForgottenFiles();
                    }}
                    disabled={forgottenFiles.page <= 1}
                  >
                    Previous
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    icon={<ChevronRight className="w-3.5 h-3.5" />}
                    onClick={() => {
                      setForgottenFilesQuery({ page: forgottenFiles.page + 1 });
                      fetchForgottenFiles();
                    }}
                    disabled={forgottenFiles.page >= forgottenFiles.totalPages}
                  >
                    Next
                  </Button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Tab 2: Optimization Blueprints */}
      {activeTab === 'blueprints' && (
        <div className="space-y-3">
          <Card padding="md" className="border-border bg-[#15171B] flex items-start gap-3">
            <ShieldCheck className="w-4 h-4 text-success flex-shrink-0 mt-0.5" />
            <div className="text-xs">
              <span className="font-semibold text-primary">Explicit User Authorization Standard:</span>
              <p className="text-secondary mt-0.5">
                DiskAtlas guarantees no automatic file purges. Every cleanup candidate requires deliberate selection and an explicit confirmation dialog before moving files to trash or executing clean commands.
              </p>
            </div>
          </Card>

          <div className="space-y-2">
            <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
              Optimization Rules & Recommended Areas
            </h2>
            <DataTable columns={recColumns} data={recommendations} />
          </div>
        </div>
      )}
    </div>
  );
};
