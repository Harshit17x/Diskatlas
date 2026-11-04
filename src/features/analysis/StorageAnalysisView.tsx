import React, { useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import {
  Film,
  FileText,
  Terminal,
  Box,
  Archive,
  Play,
} from 'lucide-react';

interface CategoryRow {
  category: string;
  extensions: string;
  fileCount: number;
  totalSize: number;
}

export const StorageAnalysisView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('categories');

  const categories = [
    { name: 'Media & Videos', icon: <Film className="w-4 h-4 text-accent" />, exts: '.mp4, .mov, .mkv, .avi', status: 'Pending scan' },
    { name: 'Developer Caches', icon: <Terminal className="w-4 h-4 text-warning" />, exts: 'node_modules, target, .cargo', status: 'Pending scan' },
    { name: 'Documents & PDFs', icon: <FileText className="w-4 h-4 text-success" />, exts: '.pdf, .docx, .xlsx, .csv', status: 'Pending scan' },
    { name: 'Installed Applications', icon: <Box className="w-4 h-4 text-primary" />, exts: '.app, .exe, AppData', status: 'Pending scan' },
    { name: 'Compressed Archives', icon: <Archive className="w-4 h-4 text-danger" />, exts: '.zip, .tar.gz, .rar, .7z', status: 'Pending scan' },
  ];

  const columns: ColumnDef<CategoryRow>[] = [
    {
      accessorKey: 'category',
      header: 'Category',
      cell: ({ row }) => <span className="font-semibold text-primary">{row.original.category}</span>,
    },
    {
      accessorKey: 'extensions',
      header: 'Mapped Extensions & Patterns',
      cell: ({ getValue }) => <span className="text-secondary font-mono text-xs">{String(getValue())}</span>,
    },
    {
      accessorKey: 'fileCount',
      header: 'Discovered Files',
      cell: ({ getValue }) => <span className="text-muted font-mono">{String(getValue())}</span>,
    },
    {
      accessorKey: 'totalSize',
      header: 'Consumed Storage',
      cell: () => <span className="text-muted font-mono">—</span>,
    },
  ];

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Storage Analysis</h1>
          <p className="text-xs text-secondary mt-0.5">
            Classify storage consumption across file types, extensions, and content categories
          </p>
        </div>

        <Button variant="primary" size="sm" icon={<Play className="w-3.5 h-3.5" />}>
          Run Category Analysis
        </Button>
      </div>

      {/* Category Overview Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {categories.map((cat) => (
          <Card key={cat.name} padding="md" hoverEffect className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="p-1.5 rounded bg-elevated border border-border">
                  {cat.icon}
                </div>
                <h3 className="text-xs font-semibold text-primary">{cat.name}</h3>
              </div>
              <Badge variant="neutral" size="sm">
                {cat.status}
              </Badge>
            </div>
            <p className="text-[11px] text-muted font-mono truncate">{cat.exts}</p>
          </Card>
        ))}
      </div>

      {/* Tabs & Table */}
      <div className="space-y-3 pt-2">
        <Tabs
          tabs={[
            { id: 'categories', label: 'Storage Categories' },
            { id: 'extensions', label: 'File Extension Breakdown' },
            { id: 'rules', label: 'Classification Rules' },
          ]}
          activeTab={activeTab}
          onChange={setActiveTab}
        />

        <DataTable
          columns={columns}
          data={[]}
          emptyMessage="No category profile has been built for the active drive yet. Trigger a scan in Phase 3 to generate classified storage breakdowns."
        />
      </div>
    </div>
  );
};
