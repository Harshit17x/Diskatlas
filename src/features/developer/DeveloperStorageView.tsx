import React from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import {
  Code2,
  Play,
  RotateCw,
} from 'lucide-react';

interface DevTarget {
  ecosystem: string;
  name: string;
  targetPattern: string;
  detectedCount: number;
  status: 'Ready to detect' | 'Searching';
}

export const DeveloperStorageView: React.FC = () => {
  const devTargets: DevTarget[] = [
    {
      ecosystem: 'Node / JavaScript',
      name: 'node_modules folders',
      targetPattern: '**/node_modules',
      detectedCount: 0,
      status: 'Ready to detect',
    },
    {
      ecosystem: 'Rust / Cargo',
      name: 'Cargo target & registry caches',
      targetPattern: '**/target, ~/.cargo/registry',
      detectedCount: 0,
      status: 'Ready to detect',
    },
    {
      ecosystem: 'Docker / Containers',
      name: 'Unused image layers & build cache',
      targetPattern: '/var/lib/docker, ~/Library/Containers/Docker',
      detectedCount: 0,
      status: 'Ready to detect',
    },
    {
      ecosystem: 'Python',
      name: 'Virtual environments & __pycache__',
      targetPattern: '**/.venv, **/__pycache__',
      detectedCount: 0,
      status: 'Ready to detect',
    },
    {
      ecosystem: 'Apple / Xcode',
      name: 'Xcode DerivedData & Archives',
      targetPattern: '~/Library/Developer/Xcode/DerivedData',
      detectedCount: 0,
      status: 'Ready to detect',
    },
    {
      ecosystem: 'Java / Android',
      name: 'Gradle caches & .m2 repository',
      targetPattern: '~/.gradle/caches, ~/.m2/repository',
      detectedCount: 0,
      status: 'Ready to detect',
    },
  ];

  const columns: ColumnDef<DevTarget>[] = [
    {
      accessorKey: 'ecosystem',
      header: 'Ecosystem',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <Code2 className="w-4 h-4 text-accent" />
          <span className="font-semibold text-primary">{row.original.ecosystem}</span>
        </div>
      ),
    },
    {
      accessorKey: 'name',
      header: 'Target Artifacts',
      cell: ({ getValue }) => <span className="text-secondary">{String(getValue())}</span>,
    },
    {
      accessorKey: 'targetPattern',
      header: 'Search Pattern',
      cell: ({ getValue }) => <span className="text-muted font-mono text-[11px]">{String(getValue())}</span>,
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ getValue }) => (
        <Badge variant="neutral" size="sm">
          {String(getValue())}
        </Badge>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* Top Title */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Developer Storage</h1>
          <p className="text-xs text-secondary mt-0.5">
            Locate heavy dependencies, transient compilation build artifacts, and package caches
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="primary" size="sm" icon={<Play className="w-3.5 h-3.5" />}>
            Scan Developer Caches
          </Button>
          <Button variant="secondary" size="sm" icon={<RotateCw className="w-3.5 h-3.5" />}>
            Configure Rules
          </Button>
        </div>
      </div>

      {/* Target Ecosystem Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {devTargets.map((target) => (
          <Card key={target.ecosystem} padding="md" hoverEffect className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold text-primary">{target.ecosystem}</span>
              <Badge variant="accent" size="sm">
                0 Items
              </Badge>
            </div>
            <p className="text-xs text-secondary">{target.name}</p>
            <p className="text-[10px] text-muted font-mono truncate">{target.targetPattern}</p>
          </Card>
        ))}
      </div>

      {/* Table view */}
      <div className="space-y-2 pt-2">
        <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
          Detected Developer Directories
        </h2>
        <DataTable
          columns={columns}
          data={devTargets}
          emptyMessage="No developer caches scanned yet. Click 'Scan Developer Caches' to locate node_modules and target artifacts."
        />
      </div>
    </div>
  );
};
