import React, { useEffect, useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import { Modal } from '../../components/common/Modal';
import type { ColumnDef } from '@tanstack/react-table';
import type { ScanSessionRecord, ScanStatus } from '../../types/bindings';
import { useHistoryStore } from '../../stores/historyStore';
import { useDriveStore } from '../../stores/driveStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  Camera,
  Clock,
  GitCompare,
  HardDrive,
  Trash2,
  Eye,
  ArrowUpRight,
  ArrowDownRight,
  Folder,
  File,
  Layers,
  ChevronLeft,
  ChevronRight,
  Plus,
  X,
  RefreshCw,
} from 'lucide-react';

export const StorageHistoryView: React.FC = () => {
  const {
    sessionsPage,
    currentPage,
    pageSize,
    filterDriveId,
    filterStatus,
    fetchSessions,
    selectedSessionDetails,
    fetchSessionDetails,
    clearSelectedSession,
    deleteSession,
    snapshots,
    fetchSnapshots,
    createSnapshot,
    baselineSessionId,
    targetSessionId,
    setBaselineSessionId,
    setTargetSessionId,
    compareScans,
    comparisonResult,
    clearComparison,
    isComparing,
  } = useHistoryStore();

  const { drives } = useDriveStore();

  const [activeDiffTab, setActiveDiffTab] = useState<'categories' | 'directories' | 'added' | 'removed' | 'modified'>('categories');
  const [isSnapshotModalOpen, setIsSnapshotModalOpen] = useState(false);
  const [snapshotDriveId, setSnapshotDriveId] = useState(drives[0]?.id || '');
  const [snapshotLabel, setSnapshotLabel] = useState('Manual Checkpoint');

  useEffect(() => {
    fetchSessions(1, 20);
    fetchSnapshots();
  }, [fetchSessions, fetchSnapshots]);

  useEffect(() => {
    if (drives.length > 0 && !snapshotDriveId) {
      setSnapshotDriveId(drives[0].id);
    }
  }, [drives, snapshotDriveId]);

  const handlePageChange = (newPage: number) => {
    fetchSessions(newPage, pageSize, filterDriveId || undefined, filterStatus || undefined);
  };

  const handleFilterDrive = (driveId: string) => {
    const d = driveId === 'ALL' ? undefined : driveId;
    fetchSessions(1, pageSize, d, filterStatus || undefined);
  };

  const handleFilterStatus = (status: string) => {
    const s = status === 'ALL' ? undefined : status;
    fetchSessions(1, pageSize, filterDriveId || undefined, s);
  };

  const handleRunComparison = async () => {
    if (baselineSessionId && targetSessionId) {
      await compareScans(baselineSessionId, targetSessionId);
    }
  };

  const handleCreateSnapshot = async () => {
    if (snapshotDriveId && snapshotLabel.trim()) {
      await createSnapshot(snapshotDriveId, snapshotLabel.trim());
      setIsSnapshotModalOpen(false);
      setSnapshotLabel('Manual Checkpoint');
    }
  };

  const getStatusBadge = (status: ScanStatus) => {
    switch (status) {
      case 'completed':
        return <Badge variant="success" size="sm">COMPLETED</Badge>;
      case 'completedWithErrors':
        return <Badge variant="warning" size="sm">ERRORS</Badge>;
      case 'interrupted':
        return <Badge variant="danger" size="sm">INTERRUPTED</Badge>;
      case 'running':
        return <Badge variant="accent" size="sm">RUNNING</Badge>;
      case 'paused':
        return <Badge variant="warning" size="sm">PAUSED</Badge>;
      case 'cancelled':
        return <Badge variant="neutral" size="sm">CANCELLED</Badge>;
      default:
        return <Badge variant="neutral" size="sm">QUEUED</Badge>;
    }
  };

  const columns: ColumnDef<ScanSessionRecord>[] = [
    {
      id: 'compareSelect',
      header: 'Compare',
      cell: ({ row }) => {
        const id = row.original.id;
        const isBaseline = baselineSessionId === id;
        const isTarget = targetSessionId === id;

        return (
          <div className="flex items-center gap-1.5">
            <button
              onClick={() => setBaselineSessionId(isBaseline ? null : id)}
              className={`px-1.5 py-0.5 text-[10px] font-mono rounded border transition-colors ${
                isBaseline
                  ? 'bg-accent text-white border-accent'
                  : 'bg-elevated text-secondary border-border hover:text-primary'
              }`}
              title="Set as Baseline scan (A)"
            >
              A
            </button>
            <button
              onClick={() => setTargetSessionId(isTarget ? null : id)}
              className={`px-1.5 py-0.5 text-[10px] font-mono rounded border transition-colors ${
                isTarget
                  ? 'bg-warning text-black border-warning font-semibold'
                  : 'bg-elevated text-secondary border-border hover:text-primary'
              }`}
              title="Set as Target scan (B)"
            >
              B
            </button>
          </div>
        );
      },
    },
    {
      accessorKey: 'id',
      header: 'Session',
      cell: ({ row }) => (
        <span className="font-mono text-xs text-primary" title={row.original.id}>
          {row.original.id.slice(0, 8)}...
        </span>
      ),
    },
    {
      accessorKey: 'driveId',
      header: 'Volume / Path',
      cell: ({ row }) => (
        <div className="truncate max-w-[200px]" title={row.original.rootPath}>
          <span className="font-mono text-xs text-accent font-medium">{row.original.driveId}</span>
          <span className="text-[11px] text-muted block truncate font-mono">{row.original.rootPath}</span>
        </div>
      ),
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ getValue }) => getStatusBadge(getValue() as ScanStatus),
    },
    {
      accessorKey: 'totalFiles',
      header: 'Files',
      cell: ({ getValue }) => (
        <span className="font-mono text-secondary text-xs">{formatNumber(Number(getValue()))}</span>
      ),
    },
    {
      accessorKey: 'totalBytes',
      header: 'Total Size',
      cell: ({ getValue }) => (
        <span className="font-mono text-primary font-medium text-xs">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'startedAt',
      header: 'Timestamp',
      cell: ({ getValue }) => (
        <div className="flex items-center gap-1.5 text-muted text-xs font-mono">
          <Clock className="w-3 h-3 text-muted shrink-0" />
          <span>{String(getValue()).replace('T', ' ').slice(0, 19)}</span>
        </div>
      ),
    },
    {
      id: 'actions',
      header: '',
      cell: ({ row }) => (
        <div className="flex items-center justify-end gap-1.5">
          <button
            onClick={() => fetchSessionDetails(row.original.id)}
            className="p-1 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors"
            title="Inspect session breakdown"
          >
            <Eye className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={() => {
              if (confirm(`Delete historical scan session ${row.original.id.slice(0, 8)}?`)) {
                deleteSession(row.original.id);
              }
            }}
            className="p-1 rounded text-muted hover:text-danger hover:bg-danger/10 transition-colors"
            title="Purge session records"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* 1. Header & Quick Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Storage History & Differential Analysis</h1>
          <p className="text-xs text-secondary mt-0.5">
            Audit historical storage growth, compute differential deltas between scans, and inspect point-in-time snapshots
          </p>
        </div>

        <div className="flex items-center gap-2">
          {baselineSessionId && targetSessionId && (
            <Button
              variant="primary"
              size="sm"
              icon={<GitCompare className="w-3.5 h-3.5" />}
              onClick={handleRunComparison}
              disabled={isComparing}
            >
              Compare Scans (A vs B)
            </Button>
          )}

          <Button
            variant="outline"
            size="sm"
            icon={<Camera className="w-3.5 h-3.5" />}
            onClick={() => setIsSnapshotModalOpen(true)}
          >
            Record Snapshot
          </Button>

          <Button
            variant="ghost"
            size="sm"
            icon={<RefreshCw className="w-3.5 h-3.5" />}
            onClick={() => fetchSessions(currentPage, pageSize)}
            title="Reload sessions"
          />
        </div>
      </div>

      {/* 2. Overview Stat Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Historical Scans</span>
            <Clock className="w-3.5 h-3.5 text-accent" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatNumber(sessionsPage?.totalCount ?? 0)}
          </div>
          <span className="text-[11px] text-muted font-mono">Persisted in SQLite</span>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Point-in-Time Snapshots</span>
            <Camera className="w-3.5 h-3.5 text-warning" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {snapshots.length}
          </div>
          <span className="text-[11px] text-muted font-mono">Storage checkpoints saved</span>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Diff Engine</span>
            <GitCompare className="w-3.5 h-3.5 text-success" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {baselineSessionId && targetSessionId ? 'Ready (A vs B)' : 'Select 2 Scans'}
          </div>
          <span className="text-[11px] text-muted font-mono">
            {baselineSessionId ? `A: ${baselineSessionId.slice(0, 6)}... ` : 'Select A '}
            {targetSessionId ? `| B: ${targetSessionId.slice(0, 6)}...` : '| Select B'}
          </span>
        </Card>
      </div>

      {/* 3. DIFFERENTIAL COMPARISON RESULT VIEW (When active) */}
      {comparisonResult && (
        <Card padding="md" className="border-accent/40 bg-[#14161A] space-y-4">
          <div className="flex items-center justify-between pb-2 border-b border-border">
            <div className="flex items-center gap-2">
              <GitCompare className="w-4 h-4 text-accent" />
              <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
                Differential Analysis: {comparisonResult.baselineSession.id.slice(0, 8)} (A) vs{' '}
                {comparisonResult.targetSession.id.slice(0, 8)} (B)
              </h2>
            </div>
            <button
              onClick={clearComparison}
              className="p-1 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors"
              title="Close comparison"
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Delta Top Cards */}
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-2 text-xs">
            <div className="p-3 bg-elevated border border-border rounded">
              <span className="text-muted block text-[10px] uppercase">Net Storage Growth</span>
              <div
                className={`text-lg font-bold font-mono mt-0.5 flex items-center gap-1 ${
                  comparisonResult.sizeDeltaBytes > 0
                    ? 'text-warning'
                    : comparisonResult.sizeDeltaBytes < 0
                    ? 'text-success'
                    : 'text-primary'
                }`}
              >
                {comparisonResult.sizeDeltaBytes > 0 ? (
                  <ArrowUpRight className="w-4 h-4" />
                ) : comparisonResult.sizeDeltaBytes < 0 ? (
                  <ArrowDownRight className="w-4 h-4" />
                ) : null}
                {formatBytes(Math.abs(comparisonResult.sizeDeltaBytes))}
              </div>
              <span className="text-[11px] text-muted">
                {comparisonResult.sizeDeltaBytes >= 0 ? 'Increase in consumed space' : 'Freed disk space'}
              </span>
            </div>

            <div className="p-3 bg-elevated border border-border rounded">
              <span className="text-muted block text-[10px] uppercase">File Count Delta</span>
              <div className="text-lg font-bold font-mono text-primary mt-0.5">
                {comparisonResult.filesDelta >= 0 ? `+${formatNumber(comparisonResult.filesDelta)}` : formatNumber(comparisonResult.filesDelta)}
              </div>
              <span className="text-[11px] text-muted">
                {comparisonResult.addedFilesCount} added • {comparisonResult.removedFilesCount} removed
              </span>
            </div>

            <div className="p-3 bg-elevated border border-border rounded">
              <span className="text-muted block text-[10px] uppercase">Modified Files</span>
              <div className="text-lg font-bold font-mono text-accent mt-0.5">
                {formatNumber(comparisonResult.modifiedFilesCount)}
              </div>
              <span className="text-[11px] text-muted">Size or timestamp changed</span>
            </div>

            <div className="p-3 bg-elevated border border-border rounded">
              <span className="text-muted block text-[10px] uppercase">Folder Shifts</span>
              <div className="text-lg font-bold font-mono text-primary mt-0.5">
                {comparisonResult.directoryDeltas.length} directories
              </div>
              <span className="text-[11px] text-muted">With measurable size changes</span>
            </div>
          </div>

          {/* Subtabs for Comparison Breakdown */}
          <div className="space-y-3">
            <div className="flex items-center gap-1 border-b border-border pb-1">
              <button
                onClick={() => setActiveDiffTab('categories')}
                className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
                  activeDiffTab === 'categories'
                    ? 'bg-accent/15 text-accent border border-accent/30'
                    : 'text-secondary hover:text-primary'
                }`}
              >
                Category Shifts ({comparisonResult.categoryDeltas.length})
              </button>
              <button
                onClick={() => setActiveDiffTab('directories')}
                className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
                  activeDiffTab === 'directories'
                    ? 'bg-accent/15 text-accent border border-accent/30'
                    : 'text-secondary hover:text-primary'
                }`}
              >
                Directory Shifts ({comparisonResult.directoryDeltas.length})
              </button>
              <button
                onClick={() => setActiveDiffTab('added')}
                className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
                  activeDiffTab === 'added'
                    ? 'bg-accent/15 text-accent border border-accent/30'
                    : 'text-secondary hover:text-primary'
                }`}
              >
                New Files ({comparisonResult.addedFilesCount})
              </button>
              <button
                onClick={() => setActiveDiffTab('removed')}
                className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
                  activeDiffTab === 'removed'
                    ? 'bg-accent/15 text-accent border border-accent/30'
                    : 'text-secondary hover:text-primary'
                }`}
              >
                Removed Files ({comparisonResult.removedFilesCount})
              </button>
              <button
                onClick={() => setActiveDiffTab('modified')}
                className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
                  activeDiffTab === 'modified'
                    ? 'bg-accent/15 text-accent border border-accent/30'
                    : 'text-secondary hover:text-primary'
                }`}
              >
                Modified Files ({comparisonResult.modifiedFilesCount})
              </button>
            </div>

            {/* Subtab 1: Categories */}
            {activeDiffTab === 'categories' && (
              <div className="border border-border rounded overflow-hidden">
                <table className="w-full text-left text-xs">
                  <thead className="bg-[#121316] text-muted border-b border-border">
                    <tr>
                      <th className="px-3 py-2">Category</th>
                      <th className="px-3 py-2">Baseline (A)</th>
                      <th className="px-3 py-2">Target (B)</th>
                      <th className="px-3 py-2">Size Delta</th>
                      <th className="px-3 py-2">File Delta</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {comparisonResult.categoryDeltas.map((cat) => (
                      <tr key={cat.category} className="hover:bg-elevated/40">
                        <td className="px-3 py-2 font-medium text-primary capitalize flex items-center gap-1.5">
                          <Layers className="w-3.5 h-3.5 text-accent" />
                          {cat.category}
                        </td>
                        <td className="px-3 py-2 font-mono text-secondary">{formatBytes(cat.baselineBytes)}</td>
                        <td className="px-3 py-2 font-mono text-secondary">{formatBytes(cat.targetBytes)}</td>
                        <td
                          className={`px-3 py-2 font-mono font-medium ${
                            cat.deltaBytes > 0 ? 'text-warning' : cat.deltaBytes < 0 ? 'text-success' : 'text-muted'
                          }`}
                        >
                          {cat.deltaBytes > 0 ? `+${formatBytes(cat.deltaBytes)}` : cat.deltaBytes < 0 ? `-${formatBytes(Math.abs(cat.deltaBytes))}` : '0 B'}
                        </td>
                        <td className="px-3 py-2 font-mono text-secondary">
                          {cat.deltaFiles > 0 ? `+${cat.deltaFiles}` : cat.deltaFiles}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {/* Subtab 2: Directory shifts */}
            {activeDiffTab === 'directories' && (
              <div className="border border-border rounded overflow-hidden">
                <table className="w-full text-left text-xs">
                  <thead className="bg-[#121316] text-muted border-b border-border">
                    <tr>
                      <th className="px-3 py-2">Directory Path</th>
                      <th className="px-3 py-2">Baseline Size</th>
                      <th className="px-3 py-2">Target Size</th>
                      <th className="px-3 py-2">Growth</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {comparisonResult.directoryDeltas.length === 0 ? (
                      <tr>
                        <td colSpan={4} className="px-3 py-4 text-center text-muted">
                          No directory size changes detected between these two scans.
                        </td>
                      </tr>
                    ) : (
                      comparisonResult.directoryDeltas.map((dir) => (
                        <tr key={dir.path} className="hover:bg-elevated/40">
                          <td className="px-3 py-2 font-mono text-primary truncate max-w-sm" title={dir.path}>
                            <Folder className="w-3.5 h-3.5 text-accent inline mr-1.5" />
                            {dir.path}
                          </td>
                          <td className="px-3 py-2 font-mono text-secondary">{formatBytes(dir.baselineSizeBytes)}</td>
                          <td className="px-3 py-2 font-mono text-secondary">{formatBytes(dir.targetSizeBytes)}</td>
                          <td
                            className={`px-3 py-2 font-mono font-medium ${
                              dir.deltaBytes > 0 ? 'text-warning' : 'text-success'
                            }`}
                          >
                            {dir.deltaBytes > 0 ? `+${formatBytes(dir.deltaBytes)}` : `-${formatBytes(Math.abs(dir.deltaBytes))}`}
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            )}

            {/* Subtab 3: Added files */}
            {activeDiffTab === 'added' && (
              <div className="border border-border rounded overflow-hidden">
                <table className="w-full text-left text-xs">
                  <thead className="bg-[#121316] text-muted border-b border-border">
                    <tr>
                      <th className="px-3 py-2">File</th>
                      <th className="px-3 py-2">Location</th>
                      <th className="px-3 py-2">Size</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {comparisonResult.addedFilesSample.length === 0 ? (
                      <tr>
                        <td colSpan={3} className="px-3 py-4 text-center text-muted">
                          No newly added files found.
                        </td>
                      </tr>
                    ) : (
                      comparisonResult.addedFilesSample.map((file) => (
                        <tr key={file.id} className="hover:bg-elevated/40">
                          <td className="px-3 py-2 font-medium text-primary">
                            <File className="w-3.5 h-3.5 text-success inline mr-1.5" />
                            {file.name}
                          </td>
                          <td className="px-3 py-2 font-mono text-secondary truncate max-w-md" title={file.path}>
                            {file.path}
                          </td>
                          <td className="px-3 py-2 font-mono text-success font-medium">{formatBytes(file.sizeBytes)}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            )}

            {/* Subtab 4: Removed files */}
            {activeDiffTab === 'removed' && (
              <div className="border border-border rounded overflow-hidden">
                <table className="w-full text-left text-xs">
                  <thead className="bg-[#121316] text-muted border-b border-border">
                    <tr>
                      <th className="px-3 py-2">File</th>
                      <th className="px-3 py-2">Previous Location</th>
                      <th className="px-3 py-2">Previous Size</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {comparisonResult.removedFilesSample.length === 0 ? (
                      <tr>
                        <td colSpan={3} className="px-3 py-4 text-center text-muted">
                          No removed files found.
                        </td>
                      </tr>
                    ) : (
                      comparisonResult.removedFilesSample.map((file) => (
                        <tr key={file.id} className="hover:bg-elevated/40">
                          <td className="px-3 py-2 font-medium text-primary">
                            <File className="w-3.5 h-3.5 text-danger inline mr-1.5" />
                            {file.name}
                          </td>
                          <td className="px-3 py-2 font-mono text-secondary truncate max-w-md" title={file.path}>
                            {file.path}
                          </td>
                          <td className="px-3 py-2 font-mono text-danger font-medium">{formatBytes(file.sizeBytes)}</td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            )}

            {/* Subtab 5: Modified files */}
            {activeDiffTab === 'modified' && (
              <div className="border border-border rounded overflow-hidden">
                <table className="w-full text-left text-xs">
                  <thead className="bg-[#121316] text-muted border-b border-border">
                    <tr>
                      <th className="px-3 py-2">File</th>
                      <th className="px-3 py-2">Location</th>
                      <th className="px-3 py-2">Baseline Size</th>
                      <th className="px-3 py-2">Target Size</th>
                      <th className="px-3 py-2">Size Delta</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {comparisonResult.modifiedFilesSample.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="px-3 py-4 text-center text-muted">
                          No modified files detected between these two scans.
                        </td>
                      </tr>
                    ) : (
                      comparisonResult.modifiedFilesSample.map((file) => (
                        <tr key={file.path} className="hover:bg-elevated/40">
                          <td className="px-3 py-2 font-medium text-primary">{file.name}</td>
                          <td className="px-3 py-2 font-mono text-secondary truncate max-w-xs" title={file.path}>
                            {file.path}
                          </td>
                          <td className="px-3 py-2 font-mono text-secondary">{formatBytes(file.baselineSizeBytes)}</td>
                          <td className="px-3 py-2 font-mono text-secondary">{formatBytes(file.targetSizeBytes)}</td>
                          <td
                            className={`px-3 py-2 font-mono font-medium ${
                              file.sizeDeltaBytes > 0 ? 'text-warning' : 'text-success'
                            }`}
                          >
                            {file.sizeDeltaBytes > 0 ? `+${formatBytes(file.sizeDeltaBytes)}` : `-${formatBytes(Math.abs(file.sizeDeltaBytes))}`}
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </Card>
      )}

      {/* 4. Filter Toolbar & Table Area */}
      <div className="space-y-3">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
              Recorded Scan Sessions
            </h2>
            {sessionsPage && (
              <Badge variant="neutral" size="sm">
                {formatNumber(sessionsPage.totalCount)} total
              </Badge>
            )}
          </div>

          {/* Filters */}
          <div className="flex items-center gap-2 text-xs">
            <div className="flex items-center gap-1.5">
              <HardDrive className="w-3.5 h-3.5 text-muted" />
              <select
                value={filterDriveId || 'ALL'}
                onChange={(e) => handleFilterDrive(e.target.value)}
                className="bg-[#15171B] border border-border rounded px-2 py-1 text-xs text-primary focus:outline-none focus:border-accent"
              >
                <option value="ALL">All Drives</option>
                {drives.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name} ({d.driveLetter || d.mountPoint})
                  </option>
                ))}
              </select>
            </div>

            <select
              value={filterStatus || 'ALL'}
              onChange={(e) => handleFilterStatus(e.target.value)}
              className="bg-[#15171B] border border-border rounded px-2 py-1 text-xs text-primary focus:outline-none focus:border-accent"
            >
              <option value="ALL">All Statuses</option>
              <option value="completed">Completed</option>
              <option value="completed_with_errors">With Errors</option>
              <option value="interrupted">Interrupted</option>
              <option value="cancelled">Cancelled</option>
            </select>

            {/* Pagination Controls */}
            {sessionsPage && sessionsPage.totalPages > 1 && (
              <div className="flex items-center gap-1 ml-2">
                <span className="text-muted font-mono text-[11px] mr-1">
                  {currentPage} / {sessionsPage.totalPages}
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  icon={<ChevronLeft className="w-3 h-3" />}
                  onClick={() => handlePageChange(currentPage - 1)}
                  disabled={currentPage <= 1}
                />
                <Button
                  size="sm"
                  variant="outline"
                  icon={<ChevronRight className="w-3 h-3" />}
                  onClick={() => handlePageChange(currentPage + 1)}
                  disabled={currentPage >= sessionsPage.totalPages}
                />
              </div>
            )}
          </div>
        </div>

        <DataTable
          columns={columns}
          data={sessionsPage?.items || []}
          emptyMessage="No historical scan sessions recorded yet. Completed scans will automatically be logged and indexed here."
        />
      </div>

      {/* 5. Session Details Modal */}
      {selectedSessionDetails && (
        <Modal
          isOpen={true}
          onClose={clearSelectedSession}
          title={`Scan Session: ${selectedSessionDetails.session.id.slice(0, 8)}...`}
          description={`Volume ${selectedSessionDetails.session.driveId} • ${selectedSessionDetails.session.rootPath}`}
          maxWidth="lg"
          footer={
            <div className="flex items-center justify-between w-full">
              <div className="flex items-center gap-2">
                <button
                  onClick={() => {
                    setBaselineSessionId(selectedSessionDetails.session.id);
                    clearSelectedSession();
                  }}
                  className="px-2.5 py-1 text-xs font-mono bg-elevated hover:bg-accent hover:text-white rounded border border-border text-secondary transition-colors"
                >
                  Set as Baseline (A)
                </button>
                <button
                  onClick={() => {
                    setTargetSessionId(selectedSessionDetails.session.id);
                    clearSelectedSession();
                  }}
                  className="px-2.5 py-1 text-xs font-mono bg-elevated hover:bg-warning hover:text-black rounded border border-border text-secondary transition-colors"
                >
                  Set as Target (B)
                </button>
              </div>
              <Button variant="ghost" onClick={clearSelectedSession}>
                Close
              </Button>
            </div>
          }
        >
          <div className="space-y-4">
            {/* Overview Stats */}
            <div className="grid grid-cols-3 gap-2 text-xs">
              <div className="p-2.5 bg-elevated border border-border rounded">
                <span className="text-muted block text-[10px] uppercase">Total Size</span>
                <span className="font-mono font-semibold text-primary text-sm mt-0.5 block">
                  {formatBytes(selectedSessionDetails.session.totalBytes)}
                </span>
              </div>
              <div className="p-2.5 bg-elevated border border-border rounded">
                <span className="text-muted block text-[10px] uppercase">Files Indexed</span>
                <span className="font-mono font-semibold text-primary text-sm mt-0.5 block">
                  {formatNumber(selectedSessionDetails.session.totalFiles)}
                </span>
              </div>
              <div className="p-2.5 bg-elevated border border-border rounded">
                <span className="text-muted block text-[10px] uppercase">Status</span>
                <div className="mt-1">{getStatusBadge(selectedSessionDetails.session.status)}</div>
              </div>
            </div>

            {/* Category Breakdown */}
            <div className="space-y-2">
              <h3 className="text-xs font-semibold text-primary uppercase tracking-wider">
                Category Distribution
              </h3>
              <div className="space-y-1.5">
                {selectedSessionDetails.categories.map((c) => (
                  <div key={c.category} className="p-2 bg-[#14161A] border border-border rounded text-xs space-y-1">
                    <div className="flex justify-between items-center">
                      <span className="font-medium text-primary capitalize">{c.category}</span>
                      <div className="font-mono text-secondary flex items-center gap-2">
                        <span>{formatBytes(c.totalBytes)}</span>
                        <span className="text-muted">({c.percentageBytes.toFixed(1)}%)</span>
                      </div>
                    </div>
                    <div className="w-full bg-[#1b1e23] h-1.5 rounded overflow-hidden">
                      <div
                        className="bg-accent h-full rounded"
                        style={{ width: `${Math.min(c.percentageBytes, 100)}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Top Directories */}
            {selectedSessionDetails.topDirectories.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-xs font-semibold text-primary uppercase tracking-wider">
                  Largest Directories
                </h3>
                <div className="border border-border rounded overflow-hidden">
                  <table className="w-full text-left text-xs">
                    <thead className="bg-[#121316] text-muted border-b border-border">
                      <tr>
                        <th className="px-3 py-1.5">Directory</th>
                        <th className="px-3 py-1.5">Files</th>
                        <th className="px-3 py-1.5">Size</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-border">
                      {selectedSessionDetails.topDirectories.map((dir) => (
                        <tr key={dir.path} className="hover:bg-elevated/40">
                          <td className="px-3 py-1.5 font-mono text-primary truncate max-w-xs" title={dir.path}>
                            {dir.name || dir.path}
                          </td>
                          <td className="px-3 py-1.5 font-mono text-secondary">{formatNumber(dir.fileCount)}</td>
                          <td className="px-3 py-1.5 font-mono text-accent font-medium">
                            {formatBytes(dir.totalSizeBytes)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </div>
        </Modal>
      )}

      {/* 6. Snapshot Creation Modal */}
      <Modal
        isOpen={isSnapshotModalOpen}
        onClose={() => setIsSnapshotModalOpen(false)}
        title="Record Storage Snapshot"
        description="Save an immutable point-in-time capacity checkpoint for historical trend analysis."
        maxWidth="md"
        footer={
          <div className="flex items-center justify-end gap-2">
            <Button variant="ghost" onClick={() => setIsSnapshotModalOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              icon={<Plus className="w-3.5 h-3.5" />}
              onClick={handleCreateSnapshot}
              disabled={!snapshotLabel.trim()}
            >
              Save Snapshot
            </Button>
          </div>
        }
      >
        <div className="space-y-3">
          <div className="space-y-1">
            <label className="text-xs font-medium text-secondary">Target Drive Volume</label>
            <select
              value={snapshotDriveId}
              onChange={(e) => setSnapshotDriveId(e.target.value)}
              className="w-full bg-[#131518] border border-border rounded px-3 py-2 text-xs text-primary focus:outline-none focus:border-accent"
            >
              {drives.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name} ({d.driveLetter || d.mountPoint})
                </option>
              ))}
            </select>
          </div>

          <div className="space-y-1">
            <label className="text-xs font-medium text-secondary">Snapshot Label</label>
            <input
              type="text"
              value={snapshotLabel}
              onChange={(e) => setSnapshotLabel(e.target.value)}
              placeholder="e.g. Pre-cleanup baseline or Weekly Snapshot"
              className="w-full bg-[#131518] border border-border rounded px-3 py-2 text-xs text-primary focus:outline-none focus:border-accent font-sans"
            />
          </div>
        </div>
      </Modal>
    </div>
  );
};
