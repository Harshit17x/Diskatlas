import React, { useEffect } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type { HiddenItem } from '../../types/bindings';
import { useAdvancedAnalysisStore } from '../../stores/advancedAnalysisStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  EyeOff,
  ShieldAlert,
  ShieldCheck,
  RotateCw,
  Search,
  ChevronLeft,
  ChevronRight,
  SlidersHorizontal,
  FolderLock,
  FileCode,
} from 'lucide-react';

export const HiddenFilesView: React.FC = () => {
  const {
    hiddenFiles,
    hiddenFilesLoading,
    hiddenFilesQuery,
    setHiddenFilesQuery,
    fetchHiddenFiles,
    selectedSessionId,
    setSelectedSessionId,
  } = useAdvancedAnalysisStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();

  useEffect(() => {
    fetchSessions(1, 20);
    fetchHiddenFiles();
  }, [fetchSessions, fetchHiddenFiles]);

  const handleSessionChange = (sid: string) => {
    setSelectedSessionId(sid || null);
    fetchHiddenFiles(sid || undefined);
  };

  const handleFilterChange = (filterType: string) => {
    setHiddenFilesQuery({ filterType, page: 1 });
    fetchHiddenFiles();
  };

  const handleSearchChange = (search: string) => {
    setHiddenFilesQuery({ search, page: 1 });
    fetchHiddenFiles();
  };

  const handlePageChange = (newPage: number) => {
    setHiddenFilesQuery({ page: newPage });
    fetchHiddenFiles();
  };

  const getAttributeBadge = (hiddenType: string) => {
    switch (hiddenType) {
      case 'system_protected':
        return <Badge variant="danger" size="sm">SYSTEM PROTECTED</Badge>;
      case 'app_hidden':
        return <Badge variant="warning" size="sm">APP DATA / CACHE</Badge>;
      case 'dotfile':
        return <Badge variant="accent" size="sm">DOTFILE</Badge>;
      default:
        return <Badge variant="neutral" size="sm">USER HIDDEN</Badge>;
    }
  };

  const getProtectionBadge = (protectionLevel: string) => {
    switch (protectionLevel) {
      case 'system_protected':
        return (
          <div className="flex items-center gap-1.5 text-danger text-xs font-medium">
            <FolderLock className="w-3.5 h-3.5 shrink-0" />
            <span>OS Protected</span>
          </div>
        );
      case 'app_managed':
        return (
          <div className="flex items-center gap-1.5 text-warning text-xs font-medium">
            <SlidersHorizontal className="w-3.5 h-3.5 shrink-0" />
            <span>App Managed</span>
          </div>
        );
      default:
        return (
          <div className="flex items-center gap-1.5 text-secondary text-xs">
            <FileCode className="w-3.5 h-3.5 shrink-0 text-muted" />
            <span>User Level</span>
          </div>
        );
    }
  };

  const columns: ColumnDef<HiddenItem>[] = [
    {
      accessorKey: 'name',
      header: 'Name',
      cell: ({ row }) => (
        <div className="flex items-center gap-2 max-w-xs">
          <EyeOff className="w-3.5 h-3.5 text-muted shrink-0" />
          <span className="font-mono text-primary font-medium truncate" title={row.original.path}>
            {row.original.name}
          </span>
        </div>
      ),
    },
    {
      accessorKey: 'hiddenType',
      header: 'Category',
      cell: ({ row }) => getAttributeBadge(row.original.hiddenType),
    },
    {
      accessorKey: 'protectionLevel',
      header: 'Protection Tier',
      cell: ({ row }) => getProtectionBadge(row.original.protectionLevel),
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Size',
      cell: ({ getValue }) => (
        <span className="font-mono text-accent font-medium text-xs">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'path',
      header: 'Absolute Location',
      cell: ({ row }) => (
        <div className="space-y-0.5 max-w-md">
          <span className="font-mono text-secondary text-[11px] truncate block" title={row.original.path}>
            {row.original.path}
          </span>
          <span className="text-[10px] text-muted block truncate italic" title={row.original.safetyNote}>
            {row.original.safetyNote}
          </span>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* 1. Header & Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Hidden & System Files Intelligence</h1>
          <p className="text-xs text-secondary mt-0.5">
            Differentiate OS kernel-protected locations from user dotfiles and application caches
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Scan Session Selector */}
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
            icon={<RotateCw className={`w-3.5 h-3.5 ${hiddenFilesLoading ? 'animate-spin' : ''}`} />}
            onClick={() => fetchHiddenFiles()}
            disabled={hiddenFilesLoading}
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* 2. Non-Destructive Protection Notice */}
      <Card padding="md" className="border-warning/30 bg-[#16181C] flex items-start gap-3">
        <ShieldAlert className="w-4 h-4 text-warning flex-shrink-0 mt-0.5" />
        <div className="text-xs space-y-0.5">
          <span className="font-semibold text-primary">Safety Policy — Read-Only Non-Destructive Inspection:</span>
          <p className="text-secondary leading-relaxed text-[11px]">
            System-protected files (such as <code>$Recycle.Bin</code>, <code>System Volume Information</code>, and core registry stores) are scanned strictly for space awareness. DiskAtlas never permits deletion or modification of operating system protected files.
          </p>
        </div>
      </Card>

      {/* 3. Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Total Hidden Storage</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatBytes(hiddenFiles?.totalBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted font-mono">
            {formatNumber(hiddenFiles?.totalItems ?? 0)} files & dirs
          </span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-danger uppercase font-semibold flex items-center gap-1.5">
            <ShieldAlert className="w-3 h-3 text-danger" />
            System Protected
          </span>
          <div className="text-xl font-bold font-mono text-danger mt-1">
            {formatBytes(hiddenFiles?.systemProtectedBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Kernel & volume reserved</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-accent uppercase font-semibold">Dotfiles & Config</span>
          <div className="text-xl font-bold font-mono text-accent mt-1">
            {formatBytes(hiddenFiles?.dotfilesBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Developer & shell states</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-success uppercase font-semibold flex items-center gap-1.5">
            <ShieldCheck className="w-3 h-3 text-success" />
            User & App Hidden
          </span>
          <div className="text-xl font-bold font-mono text-success mt-1">
            {formatBytes(hiddenFiles?.userHiddenBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">User attributes & caches</span>
        </Card>
      </div>

      {/* 4. Filter Bar & Search */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pt-2">
        <div className="flex items-center gap-1.5 overflow-x-auto">
          {[
            { id: 'all', label: 'All Hidden Items' },
            { id: 'system_protected', label: 'OS Protected Only' },
            { id: 'app_hidden', label: 'App Data & Caches' },
            { id: 'dotfile', label: 'Dotfiles Only' },
            { id: 'user_hidden', label: 'User Attribute' },
          ].map((f) => (
            <button
              key={f.id}
              onClick={() => handleFilterChange(f.id)}
              className={`px-2.5 py-1 text-xs rounded transition-colors whitespace-nowrap ${
                hiddenFilesQuery.filterType === f.id
                  ? 'bg-accent text-white font-medium'
                  : 'bg-elevated text-secondary hover:text-primary hover:bg-[#282D35]'
              }`}
            >
              {f.label}
            </button>
          ))}
        </div>

        {/* Search input */}
        <div className="relative min-w-[220px]">
          <Search className="w-3.5 h-3.5 text-muted absolute left-2.5 top-2.5 pointer-events-none" />
          <input
            type="text"
            placeholder="Search path or filename..."
            value={hiddenFilesQuery.search || ''}
            onChange={(e) => handleSearchChange(e.target.value)}
            className="w-full bg-[#15171B] border border-border rounded pl-8 pr-3 py-1.5 text-xs text-primary placeholder-muted focus:outline-none focus:border-accent font-mono"
          />
        </div>
      </div>

      {/* 5. Data Table */}
      <div className="space-y-2">
        <DataTable
          columns={columns}
          data={hiddenFiles?.items ?? []}
          emptyMessage={
            hiddenFilesLoading
              ? 'Auditing hidden attributes across scan records...'
              : 'No hidden or system files matched the active filters.'
          }
        />

        {/* Pagination Bar */}
        {hiddenFiles && hiddenFiles.totalPages > 1 && (
          <div className="flex items-center justify-between px-2 pt-2 text-xs text-secondary border-t border-border">
            <span>
              Page {hiddenFiles.page} of {hiddenFiles.totalPages} ({formatNumber(hiddenFiles.totalItems)} total items)
            </span>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                icon={<ChevronLeft className="w-3.5 h-3.5" />}
                onClick={() => handlePageChange(hiddenFiles.page - 1)}
                disabled={hiddenFiles.page <= 1}
              >
                Previous
              </Button>
              <Button
                variant="outline"
                size="sm"
                icon={<ChevronRight className="w-3.5 h-3.5" />}
                onClick={() => handlePageChange(hiddenFiles.page + 1)}
                disabled={hiddenFiles.page >= hiddenFiles.totalPages}
              >
                Next
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
