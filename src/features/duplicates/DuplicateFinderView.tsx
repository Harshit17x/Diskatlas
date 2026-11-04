import React from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import { Copy, Play, Filter } from 'lucide-react';

interface DuplicateGroup {
  hash: string;
  fileCount: number;
  fileSize: number;
  potentialReclaim: number;
  sampleName: string;
}

export const DuplicateFinderView: React.FC = () => {
  const columns: ColumnDef<DuplicateGroup>[] = [
    {
      accessorKey: 'sampleName',
      header: 'File Sample',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <Copy className="w-4 h-4 text-warning flex-shrink-0" />
          <span className="font-semibold text-primary">{row.original.sampleName}</span>
        </div>
      ),
    },
    {
      accessorKey: 'fileCount',
      header: 'Copies Found',
      cell: ({ getValue }) => <Badge variant="warning" size="sm">{String(getValue())} copies</Badge>,
    },
    {
      accessorKey: 'fileSize',
      header: 'Unit Size',
      cell: () => <span className="text-secondary font-mono">—</span>,
    },
    {
      accessorKey: 'potentialReclaim',
      header: 'Potential Reclaimable',
      cell: () => <span className="text-success font-mono font-medium">—</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Duplicate Finder</h1>
          <p className="text-xs text-secondary mt-0.5">
            Identify exact binary duplicate files using two-pass size filtering and content hashes
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="primary" size="sm" icon={<Play className="w-3.5 h-3.5" />}>
            Search Duplicates
          </Button>
          <Button variant="secondary" size="sm" icon={<Filter className="w-3.5 h-3.5" />}>
            Filters (Min Size 50MB)
          </Button>
        </div>
      </div>

      {/* Methodology Card */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Duplicate Groups</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">0</div>
          <span className="text-[11px] text-muted">Awaiting scan</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Wasted Storage</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">0 B</div>
          <span className="text-[11px] text-muted">Zero duplicates detected yet</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Hashing Engine</span>
          <div className="text-xs font-semibold text-primary mt-2">SHA-256 + Size Match</div>
          <span className="text-[11px] text-muted">Safe verification before deduplication</span>
        </Card>
      </div>

      {/* Duplicate Groups Table */}
      <div className="space-y-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Duplicate File Sets
        </h2>
        <DataTable
          columns={columns}
          data={[]}
          emptyMessage="No duplicates identified. Click 'Search Duplicates' to run candidate grouping across indexed files."
        />
      </div>
    </div>
  );
};
