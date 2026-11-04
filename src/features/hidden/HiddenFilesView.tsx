import React from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import {
  EyeOff,
  ShieldAlert,
  Play,
  RotateCw,
} from 'lucide-react';

interface HiddenItem {
  name: string;
  path: string;
  type: string;
  sizeBytes: number;
  attribute: 'System Hidden' | 'Dotfile' | 'Trash/Bin';
}

export const HiddenFilesView: React.FC = () => {
  const columns: ColumnDef<HiddenItem>[] = [
    {
      accessorKey: 'name',
      header: 'Name',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <EyeOff className="w-4 h-4 text-muted flex-shrink-0" />
          <span className="font-mono text-primary font-medium">{row.original.name}</span>
        </div>
      ),
    },
    {
      accessorKey: 'path',
      header: 'Absolute Location',
      cell: ({ getValue }) => <span className="text-secondary font-mono text-xs">{String(getValue())}</span>,
    },
    {
      accessorKey: 'attribute',
      header: 'Attribute',
      cell: ({ getValue }) => <Badge variant="warning" size="sm">{String(getValue())}</Badge>,
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Size',
      cell: () => <span className="text-muted font-mono">—</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Hidden & System Files</h1>
          <p className="text-xs text-secondary mt-0.5">
            Surface hidden attributes, operating system caches, dotfiles, and orphaned bins
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="primary" size="sm" icon={<Play className="w-3.5 h-3.5" />}>
            Scan Hidden Items
          </Button>
          <Button variant="secondary" size="sm" icon={<RotateCw className="w-3.5 h-3.5" />}>
            Refresh
          </Button>
        </div>
      </div>

      {/* Safety Policy Notice */}
      <Card padding="md" className="border-warning/30 bg-[#17191D] flex items-start gap-3">
        <ShieldAlert className="w-4 h-4 text-warning flex-shrink-0 mt-0.5" />
        <div className="text-xs">
          <span className="font-semibold text-primary">Non-Destructive Inspection Notice:</span>
          <p className="text-secondary mt-0.5">
            System files (such as <code>$Recycle.Bin</code>, <code>System Volume Information</code>, and hidden system state) are scanned exclusively in read-only mode. DiskAtlas never deletes system files automatically.
          </p>
        </div>
      </Card>

      {/* Hidden items table */}
      <div className="space-y-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Discovered Hidden Files
        </h2>
        <DataTable
          columns={columns}
          data={[]}
          emptyMessage="No hidden files indexed yet. Run 'Scan Hidden Items' to audit hidden attributes across the active drive."
        />
      </div>
    </div>
  );
};
