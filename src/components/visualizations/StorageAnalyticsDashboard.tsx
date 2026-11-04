import React, { useEffect } from 'react';
import { Card } from '../common/Card';
import { Badge } from '../common/Badge';
import { useVisualizationStore } from '../../stores/visualizationStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  PieChart,
  FileCode,
  FolderTree,
  Box,
  Terminal,
  History,
  RotateCw,
} from 'lucide-react';

export const StorageAnalyticsDashboard: React.FC = () => {
  const { analyticsData, isLoadingAnalytics, fetchAnalytics } = useVisualizationStore();

  useEffect(() => {
    fetchAnalytics();
  }, [fetchAnalytics]);

  if (isLoadingAnalytics) {
    return (
      <Card padding="lg" className="text-center py-12 space-y-2">
        <RotateCw className="w-6 h-6 text-accent animate-spin mx-auto" />
        <h3 className="text-sm font-semibold text-primary">Aggregating Storage Analytics</h3>
        <p className="text-xs text-muted max-w-sm mx-auto">
          Computing extension distribution, directory rankings, and application storage footprint...
        </p>
      </Card>
    );
  }

  if (!analyticsData) {
    return (
      <Card padding="lg" className="text-center py-12 space-y-2">
        <PieChart className="w-8 h-8 text-muted mx-auto" />
        <h3 className="text-sm font-semibold text-primary">No Analytics Data Available</h3>
        <p className="text-xs text-muted max-w-sm mx-auto">
          Complete a filesystem scan session to compute storage breakdowns.
        </p>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* 1. Category & File Type Distributions Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Category Breakdown */}
        <Card padding="md" className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
              <PieChart className="w-3.5 h-3.5 text-accent" />
              Category Storage Distribution
            </h3>
            <span className="text-[11px] text-muted">
              {analyticsData.categoryDistribution.length} categories
            </span>
          </div>

          <div className="space-y-2">
            {analyticsData.categoryDistribution.slice(0, 7).map((cat) => (
              <div key={cat.category} className="space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <span className="capitalize font-medium text-primary">
                    {cat.category.replace('_', ' ')}
                  </span>
                  <div className="flex items-center gap-2 font-mono">
                    <span className="text-secondary">{formatBytes(cat.totalBytes)}</span>
                    <span className="text-accent text-[11px] w-12 text-right">
                      {cat.percentage.toFixed(1)}%
                    </span>
                  </div>
                </div>

                <div className="w-full bg-[#131518] h-1.5 rounded overflow-hidden border border-border/50">
                  <div
                    className="bg-accent h-full rounded transition-all"
                    style={{ width: `${Math.min(cat.percentage, 100)}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </Card>

        {/* File Type / Extension Distribution */}
        <Card padding="md" className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
              <FileCode className="w-3.5 h-3.5 text-warning" />
              File Type (Extension) Distribution
            </h3>
            <span className="text-[11px] text-muted">Top 8 extensions</span>
          </div>

          <div className="space-y-2">
            {analyticsData.extensionDistribution.slice(0, 8).map((ext) => (
              <div key={ext.extension} className="space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <div className="flex items-center gap-2">
                    <Badge variant="neutral" size="sm">.{ext.extension.toUpperCase()}</Badge>
                    <span className="text-[11px] text-muted font-mono">{formatNumber(ext.fileCount)} files</span>
                  </div>
                  <div className="flex items-center gap-2 font-mono">
                    <span className="text-secondary">{formatBytes(ext.totalBytes)}</span>
                    <span className="text-warning text-[11px] w-12 text-right">
                      {ext.percentage.toFixed(1)}%
                    </span>
                  </div>
                </div>

                <div className="w-full bg-[#131518] h-1.5 rounded overflow-hidden border border-border/50">
                  <div
                    className="bg-warning h-full rounded transition-all"
                    style={{ width: `${Math.min(ext.percentage, 100)}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>

      {/* 2. Largest Directories Ranking */}
      <Card padding="md" className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
            <FolderTree className="w-3.5 h-3.5 text-accent" />
            Top Largest Storage Directories
          </h3>
          <span className="text-[11px] text-muted">Direct directory aggregation</span>
        </div>

        <div className="border border-border rounded overflow-hidden text-xs">
          <table className="w-full text-left">
            <thead className="bg-[#121316] text-muted border-b border-border">
              <tr>
                <th className="px-3 py-2">Directory Path</th>
                <th className="px-3 py-2">Files Count</th>
                <th className="px-3 py-2">Subfolders</th>
                <th className="px-3 py-2">Size</th>
                <th className="px-3 py-2 text-right">% of Disk</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {analyticsData.largestDirectories.map((dir) => (
                <tr key={dir.path} className="hover:bg-elevated/40 font-mono">
                  <td className="px-3 py-2 text-primary font-sans font-medium truncate max-w-sm" title={dir.path}>
                    {dir.path}
                  </td>
                  <td className="px-3 py-2 text-secondary">{formatNumber(dir.fileCount)}</td>
                  <td className="px-3 py-2 text-muted">{formatNumber(dir.dirCount)}</td>
                  <td className="px-3 py-2 text-accent font-semibold">{formatBytes(dir.totalBytes)}</td>
                  <td className="px-3 py-2 text-right text-secondary">{dir.percentage.toFixed(1)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      {/* 3. Application & Developer Storage Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Application Storage */}
        <Card padding="md" className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
              <Box className="w-3.5 h-3.5 text-primary" />
              Installed Application Footprint
            </h3>
            <span className="text-[11px] text-muted">{analyticsData.applicationStorage.length} apps</span>
          </div>

          {analyticsData.applicationStorage.length === 0 ? (
            <p className="text-xs text-muted py-4 text-center">
              No application ownership signatures identified in this volume.
            </p>
          ) : (
            <div className="space-y-2">
              {analyticsData.applicationStorage.slice(0, 6).map((app) => (
                <div key={app.appName} className="flex items-center justify-between text-xs py-1 border-b border-border/40">
                  <span className="font-medium text-primary">{app.appName}</span>
                  <div className="flex items-center gap-3 font-mono">
                    <span className="text-muted text-[11px]">{formatNumber(app.fileCount)} files</span>
                    <span className="text-accent font-semibold">{formatBytes(app.totalBytes)}</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>

        {/* Developer Ecosystems */}
        <Card padding="md" className="space-y-3">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
              <Terminal className="w-3.5 h-3.5 text-warning" />
              Developer Ecosystem Allocation
            </h3>
            <span className="text-[11px] text-muted">{analyticsData.developerStorage.length} ecosystems</span>
          </div>

          {analyticsData.developerStorage.length === 0 ? (
            <p className="text-xs text-muted py-4 text-center">
              No development project repositories detected on this volume.
            </p>
          ) : (
            <div className="space-y-2">
              {analyticsData.developerStorage.map((eco) => (
                <div key={eco.ecosystem} className="flex items-center justify-between text-xs py-1 border-b border-border/40">
                  <span className="font-medium text-primary">{eco.ecosystem}</span>
                  <div className="flex items-center gap-3 font-mono">
                    <span className="text-muted text-[11px]">{eco.projectCount} projects</span>
                    <span className="text-warning font-semibold">{formatBytes(eco.totalBytes)}</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>

      {/* 4. Storage History Growth Over Time */}
      <Card padding="md" className="space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
            <History className="w-3.5 h-3.5 text-secondary" />
            Storage History Snapshots Timeline
          </h3>
          <span className="text-[11px] text-muted">Point-in-time capacity checkpoints</span>
        </div>

        {analyticsData.historyTrends.length === 0 ? (
          <p className="text-xs text-muted py-4 text-center">
            No historical storage snapshots saved yet. Snapshots are created on scan completion or manually.
          </p>
        ) : (
          <div className="border border-border rounded overflow-hidden text-xs">
            <table className="w-full text-left font-mono">
              <thead className="bg-[#121316] text-muted border-b border-border">
                <tr>
                  <th className="px-3 py-2 font-sans">Date / Timestamp</th>
                  <th className="px-3 py-2 font-sans">Snapshot Label</th>
                  <th className="px-3 py-2 font-sans">Drive</th>
                  <th className="px-3 py-2 font-sans">Indexed Files</th>
                  <th className="px-3 py-2 font-sans text-right">Total Storage</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {analyticsData.historyTrends.map((snap) => (
                  <tr key={snap.id} className="hover:bg-elevated/40">
                    <td className="px-3 py-2 text-secondary">{snap.createdAt}</td>
                    <td className="px-3 py-2 text-primary font-sans font-medium">{snap.label}</td>
                    <td className="px-3 py-2 text-muted">{snap.driveId}</td>
                    <td className="px-3 py-2 text-secondary">{formatNumber(snap.totalFiles)}</td>
                    <td className="px-3 py-2 text-right text-accent font-semibold">{formatBytes(snap.totalBytes)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
};
