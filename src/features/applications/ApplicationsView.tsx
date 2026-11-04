import React from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import { Play, RotateCw, AppWindow } from 'lucide-react';

interface ApplicationRow {
  name: string;
  installPath: string;
  vendor: string;
  version: string;
  footprintBytes: number;
}

export const ApplicationsView: React.FC = () => {
  const columns: ColumnDef<ApplicationRow>[] = [
    {
      accessorKey: 'name',
      header: 'Application',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <AppWindow className="w-4 h-4 text-accent flex-shrink-0" />
          <span className="font-semibold text-primary">{row.original.name}</span>
        </div>
      ),
    },
    {
      accessorKey: 'installPath',
      header: 'Install Location',
      cell: ({ getValue }) => <span className="text-secondary font-mono text-xs">{String(getValue())}</span>,
    },
    {
      accessorKey: 'vendor',
      header: 'Vendor / Source',
      cell: ({ getValue }) => <span className="text-muted">{String(getValue())}</span>,
    },
    {
      accessorKey: 'footprintBytes',
      header: 'Storage Footprint',
      cell: () => <span className="text-muted font-mono">—</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Applications Footprint</h1>
          <p className="text-xs text-secondary mt-0.5">
            Identify storage consumed by installed software, helper services, and AppData directories
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="primary" size="sm" icon={<Play className="w-3.5 h-3.5" />}>
            Analyze Applications
          </Button>
          <Button variant="secondary" size="sm" icon={<RotateCw className="w-3.5 h-3.5" />}>
            Refresh List
          </Button>
        </div>
      </div>

      {/* Info Card */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Audited Apps</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">0</div>
          <span className="text-[11px] text-muted">Ready for enumeration</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">AppData / Support</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">—</div>
          <span className="text-[11px] text-muted">Awaiting scan</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Residual Files</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">0</div>
          <span className="text-[11px] text-muted">Orphaned configs</span>
        </Card>
      </div>

      {/* Applications Table */}
      <div className="space-y-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Software Registry & Packages
        </h2>
        <DataTable
          columns={columns}
          data={[]}
          emptyMessage="No application inventory has been indexed yet. Click 'Analyze Applications' to map software packages and their AppData footprints."
        />
      </div>
    </div>
  );
};
