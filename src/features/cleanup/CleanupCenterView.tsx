import React from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import { ShieldCheck, CheckCircle2, AlertTriangle, Play } from 'lucide-react';

interface Recommendation {
  id: string;
  title: string;
  category: 'Developer' | 'System' | 'Applications';
  safetyLevel: 'Safe' | 'Review Required';
  estimatedSavings: string;
  reason: string;
}

export const CleanupCenterView: React.FC = () => {
  const recommendations: Recommendation[] = [
    {
      id: 'rec-1',
      title: 'Stale node_modules (> 60 days unaccessed)',
      category: 'Developer',
      safetyLevel: 'Safe',
      estimatedSavings: 'Pending audit',
      reason: 'Can be reinstalled at any time via npm install / pnpm install',
    },
    {
      id: 'rec-2',
      title: 'Rust / Cargo incremental target directories',
      category: 'Developer',
      safetyLevel: 'Safe',
      estimatedSavings: 'Pending audit',
      reason: 'Recompilable build caches',
    },
    {
      id: 'rec-3',
      title: 'System temporary log dumps & crash reports',
      category: 'System',
      safetyLevel: 'Safe',
      estimatedSavings: 'Pending audit',
      reason: 'Expired diagnostic traces older than 14 days',
    },
    {
      id: 'rec-4',
      title: 'User Trash / Recycle Bin contents',
      category: 'System',
      safetyLevel: 'Review Required',
      estimatedSavings: 'Pending audit',
      reason: 'User deleted items waiting for permanent purge',
    },
  ];

  const columns: ColumnDef<Recommendation>[] = [
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
      header: 'Potential Reclaim',
      cell: ({ getValue }) => <span className="text-secondary font-mono text-xs">{String(getValue())}</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Cleanup Center</h1>
          <p className="text-xs text-secondary mt-0.5">
            Transparent, user-confirmed storage reclamation without automated background deletions
          </p>
        </div>

        <Button variant="primary" size="sm" icon={<Play className="w-3.5 h-3.5" />}>
          Run Recommendation Audit
        </Button>
      </div>

      {/* Safety Policy */}
      <Card padding="md" className="border-border bg-[#15171B] flex items-start gap-3">
        <ShieldCheck className="w-4 h-4 text-success flex-shrink-0 mt-0.5" />
        <div className="text-xs">
          <span className="font-semibold text-primary">Explicit User Authorization Standard:</span>
          <p className="text-secondary mt-0.5">
            DiskAtlas guarantees no automatic file purges. Every cleanup candidate requires deliberate selection and an explicit confirmation dialog before moving files to trash or executing clean commands.
          </p>
        </div>
      </Card>

      {/* Recommendations Table */}
      <div className="space-y-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Optimization Rules & Candidates
        </h2>
        <DataTable columns={columns} data={recommendations} />
      </div>
    </div>
  );
};
