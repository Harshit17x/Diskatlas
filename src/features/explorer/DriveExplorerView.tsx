import React, { useEffect, useState } from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { useScanStore } from '../../stores/scanStore';
import { useVisualizationStore } from '../../stores/visualizationStore';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import { TreemapChart } from '../../components/visualizations/TreemapChart';
import { SunburstChart } from '../../components/visualizations/SunburstChart';
import { StorageAnalyticsDashboard } from '../../components/visualizations/StorageAnalyticsDashboard';
import { formatBytes, formatNumber } from '../../utils/formatters';
import type { ColumnDef } from '@tanstack/react-table';
import type { ScanFileItem } from '../../types/bindings';
import {
  File,
  HardDrive,
  Play,
  RotateCw,
  FolderTree,
  ChevronRight,
  ChevronLeft,
  ChevronRight as ChevronRightIcon,
  ArrowUpDown,
  FileCode,
  Archive,
  Image,
  Video,
  Music,
  LayoutGrid,
  PieChart,
  BarChart3,
  Table,
} from 'lucide-react';

export const DriveExplorerView: React.FC = () => {
  const { drives, selectedDriveId, selectDrive } = useDriveStore();
  const selectedDrive = drives.find((d) => d.id === selectedDriveId) || drives[0];

  const [currentPath, setCurrentPath] = useState(selectedDrive?.mountPoint || '/');
  const [viewMode, setViewMode] = useState<'treemap' | 'sunburst' | 'analytics' | 'table'>('treemap');

  const {
    openScanModal,
    filePage,
    currentPage,
    pageSize,
    sortBy,
    sortDesc,
    isLoadingFiles,
    fetchScanFiles,
    activeSessionId,
    summary,
  } = useScanStore();

  const {
    tree,
    isLoadingTree,
    currentRootPath,
    breadcrumbs,
    maxDepth,
    fetchTree,
    drillDown,
    navigateUp,
    resetDrilldown,
    setMaxDepth,
  } = useVisualizationStore();

  useEffect(() => {
    fetchTree(activeSessionId || undefined, currentPath);
  }, [fetchTree, activeSessionId]);

  useEffect(() => {
    if (currentRootPath) {
      setCurrentPath(currentRootPath);
    }
  }, [currentRootPath]);

  const getFileCategoryIcon = (ext: string | null) => {
    if (!ext) return <File className="w-3.5 h-3.5 text-secondary flex-shrink-0" />;
    const lower = ext.toLowerCase();
    if (['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'bmp'].includes(lower)) {
      return <Image className="w-3.5 h-3.5 text-accent flex-shrink-0" />;
    }
    if (['mp4', 'mkv', 'mov', 'avi', 'wmv'].includes(lower)) {
      return <Video className="w-3.5 h-3.5 text-warning flex-shrink-0" />;
    }
    if (['mp3', 'wav', 'flac', 'aac', 'ogg'].includes(lower)) {
      return <Music className="w-3.5 h-3.5 text-success flex-shrink-0" />;
    }
    if (['zip', 'tar', 'gz', '7z', 'rar', 'bz2'].includes(lower)) {
      return <Archive className="w-3.5 h-3.5 text-warning flex-shrink-0" />;
    }
    if (['ts', 'tsx', 'js', 'jsx', 'rs', 'py', 'json', 'toml', 'sql'].includes(lower)) {
      return <FileCode className="w-3.5 h-3.5 text-accent flex-shrink-0" />;
    }
    return <File className="w-3.5 h-3.5 text-secondary flex-shrink-0" />;
  };

  const formatTimestamp = (ts: number | null) => {
    if (!ts) return '—';
    try {
      return new Date(ts * 1000).toLocaleString(undefined, {
        dateStyle: 'short',
        timeStyle: 'short',
      });
    } catch {
      return '—';
    }
  };

  const handleSortChange = (newSortBy: string) => {
    const isSame = sortBy === newSortBy;
    const nextDesc = isSame ? !sortDesc : true;
    fetchScanFiles(activeSessionId || undefined, 1, pageSize, undefined, newSortBy, nextDesc);
  };

  const handlePageChange = (newPage: number) => {
    fetchScanFiles(activeSessionId || undefined, newPage, pageSize, undefined, sortBy, sortDesc);
  };

  const columns: ColumnDef<ScanFileItem>[] = [
    {
      accessorKey: 'name',
      header: 'File Name',
      cell: ({ row }) => (
        <div className="flex items-center gap-2 max-w-[280px]">
          {getFileCategoryIcon(row.original.extension)}
          <span className="font-sans font-medium text-primary truncate" title={row.original.path}>
            {row.original.name}
          </span>
          {row.original.isHidden && (
            <Badge variant="warning" size="sm">
              Hidden
            </Badge>
          )}
          {row.original.isSystem && (
            <Badge variant="danger" size="sm">
              System
            </Badge>
          )}
        </div>
      ),
    },
    {
      accessorKey: 'extension',
      header: 'Ext',
      cell: ({ getValue }) => {
        const ext = getValue() as string | null;
        return ext ? (
          <Badge variant="neutral" size="sm">
            .{ext.toUpperCase()}
          </Badge>
        ) : (
          <span className="text-muted text-xs">—</span>
        );
      },
    },
    {
      accessorKey: 'sizeBytes',
      header: () => (
        <button
          onClick={() => handleSortChange('size')}
          className="flex items-center gap-1 hover:text-primary transition-colors text-xs font-semibold"
        >
          <span>Size</span>
          <ArrowUpDown className="w-3 h-3 text-muted" />
        </button>
      ),
      cell: ({ getValue }) => (
        <span className="font-mono text-accent font-medium text-xs">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'path',
      header: 'Location',
      cell: ({ getValue }) => (
        <span
          className="text-secondary font-mono text-[11px] truncate block max-w-[260px]"
          title={String(getValue())}
        >
          {String(getValue())}
        </span>
      ),
    },
    {
      accessorKey: 'modifiedAt',
      header: () => (
        <button
          onClick={() => handleSortChange('modified')}
          className="flex items-center gap-1 hover:text-primary transition-colors text-xs font-semibold"
        >
          <span>Modified</span>
          <ArrowUpDown className="w-3 h-3 text-muted" />
        </button>
      ),
      cell: ({ getValue }) => (
        <span className="text-muted text-xs font-mono">
          {formatTimestamp(getValue() as number | null)}
        </span>
      ),
    },
  ];

  const hasFiles = filePage && filePage.items.length > 0;

  return (
    <div className="space-y-4">
      {/* 1. Top Header & Volume Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Drive Explorer & Visualizer</h1>
          <p className="text-xs text-secondary mt-0.5">
            Interactive D3 Treemap, Sunburst concentric rings, and storage distribution analytics
          </p>
        </div>

        <div className="flex items-center gap-2">
          {selectedDrive && (
            <Button
              variant="outline"
              size="sm"
              icon={<RotateCw className="w-3.5 h-3.5" />}
              onClick={() => useDriveStore.getState().openDetailModal(selectedDrive.id)}
            >
              Volume Diagnostics
            </Button>
          )}
          <Button
            variant="primary"
            size="sm"
            icon={<Play className="w-3.5 h-3.5" />}
            onClick={() => openScanModal(selectedDrive?.mountPoint || drives[0]?.mountPoint || '/', selectedDrive?.id || drives[0]?.id)}
            title="Start Universal Recursive Storage Scanner"
          >
            Scan Root Path
          </Button>
        </div>
      </div>

      {/* 2. Target Volume Card */}
      <Card padding="md" className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <HardDrive className="w-4 h-4 text-accent" />
            <span className="text-xs font-semibold text-primary">Target Volume:</span>
            <select
              value={selectedDrive?.id || ''}
              onChange={(e) => {
                selectDrive(e.target.value);
                const d = drives.find((drv) => drv.id === e.target.value);
                if (d) {
                  setCurrentPath(d.mountPoint);
                  fetchTree(activeSessionId || undefined, d.mountPoint);
                }
              }}
              className="bg-[#15171B] border border-border rounded px-2.5 py-1 text-xs text-primary font-mono focus:outline-none focus:border-accent"
            >
              {drives.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name} {d.driveLetter ? `[${d.driveLetter}]` : `(${d.mountPoint})`} —{' '}
                  {d.totalBytes !== null ? formatBytes(d.totalBytes) : 'No Media'}
                </option>
              ))}
            </select>
          </div>

          <div className="text-xs text-muted font-mono">
            Filesystem: <span className="text-secondary">{selectedDrive?.fileSystem || 'NTFS/APFS'}</span>
          </div>
        </div>

        {/* Current Path Bar */}
        <div className="flex items-center justify-between px-3 py-2 bg-[#14161A] border border-border rounded text-xs font-mono text-secondary">
          <div className="flex items-center gap-1.5 truncate">
            <FolderTree className="w-3.5 h-3.5 text-muted flex-shrink-0" />
            <span className="text-primary font-semibold truncate">{currentPath}</span>
            <ChevronRight className="w-3 h-3 text-muted flex-shrink-0" />
            <span className="text-muted text-[11px] font-sans truncate">
              {summary
                ? `Indexed ${formatNumber(summary.totalFiles)} files (${formatBytes(summary.totalBytes)})`
                : 'Recursive scanner ready'}
            </span>
          </div>
          {summary && (
            <Badge variant="accent" size="sm">
              Session {summary.sessionId.slice(0, 8)}
            </Badge>
          )}
        </div>
      </Card>

      {/* 3. View Mode Switcher */}
      <div className="flex items-center justify-between gap-2 border-b border-border pb-2">
        <div className="flex items-center gap-1">
          {[
            { id: 'treemap', label: 'D3 Treemap', icon: <LayoutGrid className="w-3.5 h-3.5" /> },
            { id: 'sunburst', label: 'D3 Sunburst', icon: <PieChart className="w-3.5 h-3.5" /> },
            { id: 'analytics', label: 'Analytics Distribution', icon: <BarChart3 className="w-3.5 h-3.5" /> },
            { id: 'table', label: 'File Registry Table', icon: <Table className="w-3.5 h-3.5" /> },
          ].map((mode) => (
            <button
              key={mode.id}
              onClick={() => setViewMode(mode.id as any)}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-xs rounded transition-colors ${
                viewMode === mode.id
                  ? 'bg-accent text-white font-medium'
                  : 'text-secondary hover:text-primary hover:bg-[#1C1F25]'
              }`}
            >
              {mode.icon}
              <span>{mode.label}</span>
            </button>
          ))}
        </div>

        <span className="text-[11px] text-muted italic hidden sm:inline">
          Database Aggregation • Zero Synthetic Data
        </span>
      </div>

      {/* 4. Active Visualization / Table View */}
      {viewMode === 'treemap' && (
        <div>
          {isLoadingTree ? (
            <Card padding="lg" className="text-center py-12 space-y-2">
              <RotateCw className="w-6 h-6 text-accent animate-spin mx-auto" />
              <h3 className="text-sm font-semibold text-primary">Aggregating Treemap Hierarchy</h3>
              <p className="text-xs text-muted max-w-sm mx-auto">
                Computing database tile squarification and directory depth boundaries...
              </p>
            </Card>
          ) : !tree ? (
            <Card padding="lg" className="text-center py-12 space-y-2">
              <LayoutGrid className="w-8 h-8 text-muted mx-auto" />
              <h3 className="text-sm font-semibold text-primary">No Scan Tree Available</h3>
              <p className="text-xs text-muted max-w-sm mx-auto">
                Run a scan on this volume to generate the interactive Treemap.
              </p>
            </Card>
          ) : (
            <TreemapChart
              data={tree}
              onDrillDown={drillDown}
              onNavigateUp={navigateUp}
              onReset={resetDrilldown}
              breadcrumbs={breadcrumbs}
              maxDepth={maxDepth}
              onSetMaxDepth={setMaxDepth}
            />
          )}
        </div>
      )}

      {viewMode === 'sunburst' && (
        <div>
          {isLoadingTree ? (
            <Card padding="lg" className="text-center py-12 space-y-2">
              <RotateCw className="w-6 h-6 text-accent animate-spin mx-auto" />
              <h3 className="text-sm font-semibold text-primary">Aggregating Sunburst Ring Partitions</h3>
              <p className="text-xs text-muted max-w-sm mx-auto">
                Calculating angular segments and concentric ring coordinates...
              </p>
            </Card>
          ) : !tree ? (
            <Card padding="lg" className="text-center py-12 space-y-2">
              <PieChart className="w-8 h-8 text-muted mx-auto" />
              <h3 className="text-sm font-semibold text-primary">No Scan Tree Available</h3>
              <p className="text-xs text-muted max-w-sm mx-auto">
                Run a scan on this volume to generate the Sunburst concentric rings.
              </p>
            </Card>
          ) : (
            <SunburstChart
              data={tree}
              onDrillDown={drillDown}
              onNavigateUp={navigateUp}
              onReset={resetDrilldown}
              breadcrumbs={breadcrumbs}
            />
          )}
        </div>
      )}

      {viewMode === 'analytics' && <StorageAnalyticsDashboard />}

      {viewMode === 'table' && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
                Discovered File Registry
              </h2>
              {filePage && (
                <Badge variant="neutral" size="sm">
                  {formatNumber(filePage.totalCount)} records
                </Badge>
              )}
            </div>

            {hasFiles && (
              <div className="flex items-center gap-2 text-xs">
                <span className="text-muted font-mono text-[11px]">
                  Page {currentPage} of {filePage.totalPages || 1}
                </span>
                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="outline"
                    icon={<ChevronLeft className="w-3 h-3" />}
                    onClick={() => handlePageChange(currentPage - 1)}
                    disabled={currentPage <= 1 || isLoadingFiles}
                  >
                    Prev
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    icon={<ChevronRightIcon className="w-3 h-3" />}
                    onClick={() => handlePageChange(currentPage + 1)}
                    disabled={currentPage >= (filePage?.totalPages || 1) || isLoadingFiles}
                  >
                    Next
                  </Button>
                </div>
              </div>
            )}
          </div>

          <DataTable
            columns={columns}
            data={filePage?.items || []}
            emptyMessage="No filesystem scan has been initiated yet. Click 'Scan Root Path' to trigger the Universal Filesystem Scanner."
          />
        </div>
      )}
    </div>
  );
};
