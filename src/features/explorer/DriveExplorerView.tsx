import React, { useState } from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import { formatBytes } from '../../utils/formatters';
import type { ColumnDef } from '@tanstack/react-table';
import {
  Folder,
  File,
  HardDrive,
  Play,
  RotateCw,
  FolderTree,
  ChevronRight,
} from 'lucide-react';

interface FileNode {
  name: string;
  path: string;
  kind: 'directory' | 'file';
  sizeBytes: number;
  extension?: string;
  modified: string;
}

export const DriveExplorerView: React.FC = () => {
  const { drives, selectedDriveId, selectDrive } = useDriveStore();
  const selectedDrive = drives.find((d) => d.id === selectedDriveId) || drives[0];

  const [currentPath, setCurrentPath] = useState(selectedDrive?.mountPoint || '/');

  // Realistic empty file explorer placeholder dataset (without fabricated random scan results)
  const columns: ColumnDef<FileNode>[] = [
    {
      accessorKey: 'name',
      header: 'Name',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          {row.original.kind === 'directory' ? (
            <Folder className="w-4 h-4 text-accent flex-shrink-0" />
          ) : (
            <File className="w-4 h-4 text-secondary flex-shrink-0" />
          )}
          <span className="font-sans font-medium text-primary">{row.original.name}</span>
        </div>
      ),
    },
    {
      accessorKey: 'kind',
      header: 'Type',
      cell: ({ getValue }) => (
        <Badge variant={getValue() === 'directory' ? 'accent' : 'neutral'} size="sm">
          {String(getValue()).toUpperCase()}
        </Badge>
      ),
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Size',
      cell: ({ getValue }) => (
        <span className="text-secondary font-mono">{formatBytes(Number(getValue()))}</span>
      ),
    },
    {
      accessorKey: 'modified',
      header: 'Modified',
      cell: ({ getValue }) => <span className="text-muted text-xs">{String(getValue())}</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* Top Header & Breadcrumb */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Drive Explorer</h1>
          <p className="text-xs text-secondary mt-0.5">
            Inspect directory structures, folder weights, and volume allocations
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="primary"
            size="sm"
            icon={<Play className="w-3.5 h-3.5" />}
            onClick={() => {}}
            title="Starts Rayon high-throughput filesystem scanner (Staged for Phase 3)"
          >
            Scan Root Path
          </Button>
          <Button variant="secondary" size="sm" icon={<RotateCw className="w-3.5 h-3.5" />}>
            Reload
          </Button>
        </div>
      </div>

      {/* Selected Volume Card & Path Navigation */}
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
                if (d) setCurrentPath(d.mountPoint);
              }}
              className="bg-[#15171B] border border-border rounded px-2.5 py-1 text-xs text-primary font-mono focus:outline-none focus:border-accent"
            >
              {drives.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.label} ({d.mountPoint}) — {formatBytes(d.totalBytes)}
                </option>
              ))}
            </select>
          </div>

          <div className="text-xs text-muted font-mono">
            Filesystem: <span className="text-secondary">{selectedDrive?.fileSystem || 'NTFS/APFS'}</span>
          </div>
        </div>

        {/* Path Breadcrumbs */}
        <div className="flex items-center gap-1.5 px-3 py-2 bg-[#14161A] border border-border rounded text-xs font-mono text-secondary">
          <FolderTree className="w-3.5 h-3.5 text-muted flex-shrink-0" />
          <span className="text-primary font-semibold">{currentPath}</span>
          <ChevronRight className="w-3 h-3 text-muted" />
          <span className="text-muted italic text-[11px] font-sans">
            (Scanner indexed state will populate here)
          </span>
        </div>
      </Card>

      {/* TanStack Table Data Area */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
            Directory Contents
          </h2>
          <span className="text-[11px] text-muted font-mono">Ready for indexing</span>
        </div>

        <DataTable
          columns={columns}
          data={[]}
          emptyMessage="No directory scan has been initiated yet. Click 'Scan Root Path' to trigger the Phase 3 high-throughput traversal engine."
        />
      </div>
    </div>
  );
};
