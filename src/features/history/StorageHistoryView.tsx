import React from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import { Camera, Clock } from 'lucide-react';

interface ScanSessionRecord {
  id: string;
  driveId: string;
  rootPath: string;
  status: string;
  totalFiles: number;
  totalBytes: number;
  startedAt: string;
}

export const StorageHistoryView: React.FC = () => {
  const columns: ColumnDef<ScanSessionRecord>[] = [
    {
      accessorKey: 'id',
      header: 'Session ID',
      cell: ({ getValue }) => <span className="font-mono text-xs text-primary">{String(getValue()).slice(0, 8)}...</span>,
    },
    {
      accessorKey: 'driveId',
      header: 'Drive',
      cell: ({ getValue }) => <span className="font-mono text-xs text-accent">{String(getValue())}</span>,
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ getValue }) => (
        <Badge variant={getValue() === 'completed' ? 'success' : 'neutral'} size="sm">
          {String(getValue()).toUpperCase()}
        </Badge>
      ),
    },
    {
      accessorKey: 'totalFiles',
      header: 'Files Indexed',
      cell: ({ getValue }) => <span className="font-mono text-secondary text-xs">{String(getValue())}</span>,
    },
    {
      accessorKey: 'startedAt',
      header: 'Date & Time',
      cell: ({ getValue }) => (
        <div className="flex items-center gap-1.5 text-muted text-xs font-mono">
          <Clock className="w-3 h-3 text-muted" />
          <span>{String(getValue())}</span>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Storage History & Snapshots</h1>
          <p className="text-xs text-secondary mt-0.5">
            Audit historical storage growth, differential changes between scans, and volume utilization over time
          </p>
        </div>

        <Button variant="primary" size="sm" icon={<Camera className="w-3.5 h-3.5" />}>
          Record Storage Snapshot
        </Button>
      </div>

      {/* Snapshot Statistics */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Saved Snapshots</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">0</div>
          <span className="text-[11px] text-muted">Baseline history</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Net Growth Rate</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">0.0 MB / day</div>
          <span className="text-[11px] text-muted">Awaiting successive scans</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Historical Audit Log</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">Active</div>
          <span className="text-[11px] text-muted">Persisted in SQLite database</span>
        </Card>
      </div>

      {/* Audit Log Table */}
      <div className="space-y-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Recent Scan Sessions
        </h2>
        <DataTable
          columns={columns}
          data={[]}
          emptyMessage="No historical scan sessions recorded yet in SQLite. Sessions will automatically populate here upon completing disk analyses."
        />
      </div>
    </div>
  );
};
