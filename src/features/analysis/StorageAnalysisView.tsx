import React, { useEffect, useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import { Modal } from '../../components/common/Modal';
import type { ColumnDef } from '@tanstack/react-table';
import type { ScanFileItem } from '../../types/bindings';
import { useAnalysisStore } from '../../stores/analysisStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  Film,
  FileText,
  Terminal,
  Box,
  Archive,
  RotateCw,
  Server,
  Compass,
  Cloud,
  Gamepad2,
  Shield,
  Cpu,
  EyeOff,
  HelpCircle,
  Clock,
  Music,
  Image,
  ChevronLeft,
  ChevronRight,
  Plus,
  Trash2,
  Info,
  FolderTree,
} from 'lucide-react';

export const StorageAnalysisView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('categories');
  const [isOverrideModalOpen, setIsOverrideModalOpen] = useState(false);
  const [overridePattern, setOverridePattern] = useState('');
  const [overrideCategory, setOverrideCategory] = useState('developer');
  const [overrideSubcategory, setOverrideSubcategory] = useState('');

  const {
    activeProfile,
    selectedCategory,
    categoryFilePage,
    currentPage,
    pageSize,
    overrides,
    isLoading,
    isLoadingFiles,
    fetchAnalysis,
    runClassification,
    fetchCategoryFiles,
    selectCategory,
    fetchOverrides,
    addOverride,
    deleteOverride,
  } = useAnalysisStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();

  useEffect(() => {
    fetchSessions(1, 20);
    fetchOverrides();
    fetchAnalysis();
  }, [fetchSessions, fetchOverrides, fetchAnalysis]);

  const handleSelectSession = (sid: string) => {
    fetchAnalysis(sid);
  };

  const handleAddOverride = async () => {
    if (overridePattern.trim()) {
      await addOverride(overridePattern.trim(), overrideCategory, overrideSubcategory.trim() || undefined);
      setIsOverrideModalOpen(false);
      setOverridePattern('');
      setOverrideSubcategory('');
      if (activeProfile?.sessionId) {
        runClassification(activeProfile.sessionId);
      }
    }
  };

  const getCategoryIcon = (id: string) => {
    switch (id) {
      case 'os':
        return <Server className="w-4 h-4 text-accent" />;
      case 'applications':
        return <Box className="w-4 h-4 text-primary" />;
      case 'developer':
        return <Terminal className="w-4 h-4 text-warning" />;
      case 'temporary':
        return <Clock className="w-4 h-4 text-danger" />;
      case 'documents':
        return <FileText className="w-4 h-4 text-success" />;
      case 'images':
        return <Image className="w-4 h-4 text-accent" />;
      case 'videos':
        return <Film className="w-4 h-4 text-warning" />;
      case 'audio':
        return <Music className="w-4 h-4 text-success" />;
      case 'archives':
        return <Archive className="w-4 h-4 text-danger" />;
      case 'browsers':
        return <Compass className="w-4 h-4 text-accent" />;
      case 'cloud':
        return <Cloud className="w-4 h-4 text-accent" />;
      case 'gaming':
        return <Gamepad2 className="w-4 h-4 text-warning" />;
      case 'backups':
        return <Shield className="w-4 h-4 text-success" />;
      case 'vm_emulators':
        return <Cpu className="w-4 h-4 text-accent" />;
      case 'hidden':
        return <EyeOff className="w-4 h-4 text-muted" />;
      default:
        return <HelpCircle className="w-4 h-4 text-muted" />;
    }
  };

  const getConfidenceBadge = (confidence: number) => {
    if (confidence >= 0.85) {
      return <Badge variant="success" size="sm">HIGH ({(confidence * 100).toFixed(0)}%)</Badge>;
    }
    if (confidence >= 0.6) {
      return <Badge variant="accent" size="sm">MED ({(confidence * 100).toFixed(0)}%)</Badge>;
    }
    return <Badge variant="neutral" size="sm">LOW ({(confidence * 100).toFixed(0)}%)</Badge>;
  };

  const fileColumns: ColumnDef<ScanFileItem>[] = [
    {
      accessorKey: 'name',
      header: 'File Name',
      cell: ({ row }) => (
        <span className="font-sans font-medium text-primary truncate block max-w-xs" title={row.original.path}>
          {row.original.name}
        </span>
      ),
    },
    {
      accessorKey: 'extension',
      header: 'Ext',
      cell: ({ getValue }) => {
        const ext = getValue() as string | null;
        return ext ? (
          <Badge variant="neutral" size="sm">.{ext.toUpperCase()}</Badge>
        ) : (
          <span className="text-muted text-xs">—</span>
        );
      },
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
      header: 'Full Location',
      cell: ({ getValue }) => (
        <span className="font-mono text-secondary text-[11px] truncate block max-w-md" title={String(getValue())}>
          {String(getValue())}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* 1. Header & Controls */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Deterministic Storage Classification</h1>
          <p className="text-xs text-secondary mt-0.5">
            Rule-based categorization across 16 mutually exclusive storage profiles with strict confidence tracking
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Target Session Selector */}
          <select
            value={activeProfile?.sessionId || ''}
            onChange={(e) => handleSelectSession(e.target.value)}
            className="bg-[#15171B] border border-border rounded px-2.5 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          >
            {sessionsPage?.items.map((s) => (
              <option key={s.id} value={s.id}>
                Session {s.id.slice(0, 8)} ({s.driveId} • {formatBytes(s.totalBytes)})
              </option>
            ))}
          </select>

          {activeProfile && (
            <Button
              variant="outline"
              size="sm"
              icon={<RotateCw className="w-3.5 h-3.5" />}
              onClick={() => runClassification(activeProfile.sessionId)}
              disabled={isLoading}
              title="Re-run deterministic classification engine"
            >
              Re-Classify
            </Button>
          )}

          <Button
            variant="primary"
            size="sm"
            icon={<Plus className="w-3.5 h-3.5" />}
            onClick={() => setIsOverrideModalOpen(true)}
          >
            Add Rule
          </Button>
        </div>
      </div>

      {/* 2. Safety & Engineering Principles Alert Banner */}
      <div className="p-3 bg-[#131518] border border-border rounded flex items-start gap-2.5 text-xs text-secondary">
        <Info className="w-4 h-4 text-accent shrink-0 mt-0.5" />
        <div className="space-y-0.5 text-[11px]">
          <span className="font-semibold text-primary block">Deterministic Classification Principles:</span>
          <p>
            100% of discovered storage is strictly accounted for without double-counting.
            A hidden file is not automatically unnecessary; temporary caches are not automatically safe to delete; system-protected locations are strictly guarded.
          </p>
        </div>
      </div>

      {/* 3. Top Metrics Banner */}
      {activeProfile && (
        <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
          <Card padding="md">
            <span className="text-[11px] text-muted uppercase font-semibold">Classified Storage</span>
            <div className="text-xl font-bold font-mono text-primary mt-1">
              {formatBytes(activeProfile.totalBytes)}
            </div>
            <span className="text-[11px] text-accent font-mono">
              {activeProfile.classifiedPercent.toFixed(1)}% identified
            </span>
          </Card>

          <Card padding="md">
            <span className="text-[11px] text-muted uppercase font-semibold">Indexed Files</span>
            <div className="text-xl font-bold font-mono text-primary mt-1">
              {formatNumber(activeProfile.totalFiles)}
            </div>
            <span className="text-[11px] text-muted font-mono">16 distinct categories</span>
          </Card>

          <Card padding="md">
            <span className="text-[11px] text-muted uppercase font-semibold">Engine Confidence</span>
            <div className="text-xl font-bold font-mono text-success mt-1">
              {(activeProfile.overallConfidence * 100).toFixed(0)}%
            </div>
            <span className="text-[11px] text-muted font-mono">Deterministic heuristics</span>
          </Card>

          <Card padding="md">
            <span className="text-[11px] text-muted uppercase font-semibold">Target Volume</span>
            <div className="text-xl font-bold font-mono text-primary mt-1 truncate">
              {activeProfile.driveId}
            </div>
            <span className="text-[11px] text-muted font-mono">Session {activeProfile.sessionId.slice(0, 8)}</span>
          </Card>
        </div>
      )}

      {/* 4. Tabs Area */}
      <div className="space-y-3 pt-1">
        <Tabs
          tabs={[
            { id: 'categories', label: `16 Storage Categories (${activeProfile?.categories.length ?? 0})` },
            { id: 'drilldown', label: selectedCategory ? `Drilldown: ${selectedCategory.toUpperCase()}` : 'Path Drilldown' },
            { id: 'overrides', label: `Override Rules (${overrides.length})` },
          ]}
          activeTab={activeTab}
          onChange={setActiveTab}
        />

        {/* Tab 1: Categories Cards Grid */}
        {activeTab === 'categories' && (
          <div className="space-y-4">
            {!activeProfile ? (
              <Card padding="lg" className="text-center py-10 space-y-2">
                <FolderTree className="w-8 h-8 text-muted mx-auto" />
                <h3 className="text-sm font-semibold text-primary">No Scan Selected for Analysis</h3>
                <p className="text-xs text-muted max-w-sm mx-auto">
                  Run a filesystem scan or select a completed scan session from the dropdown above to inspect classified storage categories.
                </p>
              </Card>
            ) : (
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
                {activeProfile.categories.map((cat) => {
                  const isSelected = selectedCategory === cat.category;
                  return (
                    <Card
                      key={cat.category}
                      padding="md"
                      hoverEffect
                      selected={isSelected}
                      onClick={() => {
                        selectCategory(cat.category);
                        setActiveTab('drilldown');
                      }}
                      className="cursor-pointer space-y-3 flex flex-col justify-between"
                    >
                      <div>
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <div className="p-1.5 rounded bg-elevated border border-border">
                              {getCategoryIcon(cat.category)}
                            </div>
                            <h3 className="text-xs font-semibold text-primary">{cat.label}</h3>
                          </div>
                          {getConfidenceBadge(cat.avgConfidence)}
                        </div>

                        <div className="mt-3">
                          <div className="text-lg font-bold font-mono text-primary">
                            {formatBytes(cat.totalBytes)}
                          </div>
                          <div className="flex justify-between items-center text-[11px] text-muted font-mono mt-0.5">
                            <span>{formatNumber(cat.fileCount)} files</span>
                            <span className="text-accent">{cat.percentageBytes.toFixed(1)}%</span>
                          </div>
                        </div>

                        {/* Progress Bar */}
                        <div className="w-full bg-[#131518] h-1.5 rounded overflow-hidden mt-2 border border-border/50">
                          <div
                            className="bg-accent h-full rounded transition-all"
                            style={{ width: `${Math.min(cat.percentageBytes, 100)}%` }}
                          />
                        </div>
                      </div>

                      {/* Subcategory & App Preview */}
                      <div className="pt-2 border-t border-border/60 text-[11px] text-muted space-y-1">
                        {cat.subcategories.length > 0 ? (
                          <div className="truncate">
                            <span className="text-secondary font-medium">Sub:</span> {cat.subcategories.map((s) => s.subcategory).join(', ')}
                          </div>
                        ) : null}
                        {cat.topApps.length > 0 ? (
                          <div className="truncate">
                            <span className="text-secondary font-medium">Apps:</span> {cat.topApps.map((a) => a.appName).join(', ')}
                          </div>
                        ) : null}
                      </div>
                    </Card>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {/* Tab 2: Path Drilldown Table */}
        {activeTab === 'drilldown' && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
                  Category Files: {selectedCategory ? selectedCategory.toUpperCase() : 'None Selected'}
                </h2>
                {categoryFilePage && (
                  <Badge variant="neutral" size="sm">
                    {formatNumber(categoryFilePage.totalCount)} files
                  </Badge>
                )}
              </div>

              {categoryFilePage && categoryFilePage.totalPages > 1 && (
                <div className="flex items-center gap-1">
                  <span className="text-muted font-mono text-[11px] mr-1">
                    Page {currentPage} of {categoryFilePage.totalPages}
                  </span>
                  <Button
                    size="sm"
                    variant="outline"
                    icon={<ChevronLeft className="w-3 h-3" />}
                    onClick={() => {
                      if (activeProfile?.sessionId && selectedCategory) {
                        fetchCategoryFiles(activeProfile.sessionId, selectedCategory, currentPage - 1, pageSize);
                      }
                    }}
                    disabled={currentPage <= 1 || isLoadingFiles}
                  />
                  <Button
                    size="sm"
                    variant="outline"
                    icon={<ChevronRight className="w-3 h-3" />}
                    onClick={() => {
                      if (activeProfile?.sessionId && selectedCategory) {
                        fetchCategoryFiles(activeProfile.sessionId, selectedCategory, currentPage + 1, pageSize);
                      }
                    }}
                    disabled={currentPage >= categoryFilePage.totalPages || isLoadingFiles}
                  />
                </div>
              )}
            </div>

            <DataTable
              columns={fileColumns}
              data={categoryFilePage?.items || []}
              emptyMessage={
                selectedCategory
                  ? 'No files found in this category for the selected session.'
                  : 'Select a storage category card from the 16 Storage Categories tab to inspect exact file paths.'
              }
            />
          </div>
        )}

        {/* Tab 3: Classification Rules & Overrides */}
        {activeTab === 'overrides' && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
                  Active Override Rules
                </h2>
                <p className="text-[11px] text-muted">
                  Rules take highest priority (Confidence 1.0) and map specific paths or subpaths to user-defined categories.
                </p>
              </div>
              <Button
                size="sm"
                variant="primary"
                icon={<Plus className="w-3.5 h-3.5" />}
                onClick={() => setIsOverrideModalOpen(true)}
              >
                New Rule
              </Button>
            </div>

            <div className="border border-border rounded overflow-hidden">
              <table className="w-full text-left text-xs">
                <thead className="bg-[#121316] text-muted border-b border-border">
                  <tr>
                    <th className="px-3 py-2">Pattern / Subpath</th>
                    <th className="px-3 py-2">Target Category</th>
                    <th className="px-3 py-2">Subcategory</th>
                    <th className="px-3 py-2">Created</th>
                    <th className="px-3 py-2 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {overrides.length === 0 ? (
                    <tr>
                      <td colSpan={5} className="px-3 py-4 text-center text-muted">
                        No manual override rules created yet. Built-in deterministic rules are actively classifying storage.
                      </td>
                    </tr>
                  ) : (
                    overrides.map((rule) => (
                      <tr key={rule.id} className="hover:bg-elevated/40">
                        <td className="px-3 py-2 font-mono text-primary font-medium">{rule.pattern}</td>
                        <td className="px-3 py-2 capitalize">
                          <Badge variant="accent" size="sm">{rule.category}</Badge>
                        </td>
                        <td className="px-3 py-2 text-secondary font-mono">{rule.subcategory || '—'}</td>
                        <td className="px-3 py-2 text-muted font-mono">{rule.createdAt.slice(0, 10)}</td>
                        <td className="px-3 py-2 text-right">
                          <button
                            onClick={() => deleteOverride(rule.id)}
                            className="p-1 rounded text-muted hover:text-danger hover:bg-danger/10 transition-colors"
                            title="Delete rule"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>

      {/* 5. Add Override Rule Modal */}
      <Modal
        isOpen={isOverrideModalOpen}
        onClose={() => setIsOverrideModalOpen(false)}
        title="Add Storage Category Override Rule"
        description="Override default heuristics by mapping a specific path or substring to a chosen category with 100% confidence."
        maxWidth="md"
        footer={
          <div className="flex items-center justify-end gap-2">
            <Button variant="ghost" onClick={() => setIsOverrideModalOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleAddOverride}
              disabled={!overridePattern.trim()}
            >
              Save Rule
            </Button>
          </div>
        }
      >
        <div className="space-y-3">
          <div className="space-y-1">
            <label className="text-xs font-medium text-secondary">Path Pattern / Substring</label>
            <input
              type="text"
              value={overridePattern}
              onChange={(e) => setOverridePattern(e.target.value)}
              placeholder="e.g. /my-custom-builds or D:\Databases"
              className="w-full bg-[#131518] border border-border rounded px-3 py-2 text-xs text-primary focus:outline-none focus:border-accent font-mono"
            />
          </div>

          <div className="space-y-1">
            <label className="text-xs font-medium text-secondary">Target Category</label>
            <select
              value={overrideCategory}
              onChange={(e) => setOverrideCategory(e.target.value)}
              className="w-full bg-[#131518] border border-border rounded px-3 py-2 text-xs text-primary focus:outline-none focus:border-accent"
            >
              <option value="os">Operating System</option>
              <option value="applications">Applications</option>
              <option value="developer">Developer Storage</option>
              <option value="temporary">Temporary Files</option>
              <option value="documents">Personal Documents</option>
              <option value="images">Images</option>
              <option value="videos">Videos</option>
              <option value="audio">Audio</option>
              <option value="archives">Downloads & Archives</option>
              <option value="browsers">Browser Storage</option>
              <option value="cloud">Cloud Synchronization</option>
              <option value="gaming">Gaming</option>
              <option value="backups">Backups & Recovery</option>
              <option value="vm_emulators">Virtual Machines & Emulators</option>
              <option value="hidden">Hidden Storage</option>
              <option value="unknown">Unknown Storage</option>
            </select>
          </div>

          <div className="space-y-1">
            <label className="text-xs font-medium text-secondary">Custom Subcategory Label (Optional)</label>
            <input
              type="text"
              value={overrideSubcategory}
              onChange={(e) => setOverrideSubcategory(e.target.value)}
              placeholder="e.g. Experimental Models"
              className="w-full bg-[#131518] border border-border rounded px-3 py-2 text-xs text-primary focus:outline-none focus:border-accent font-sans"
            />
          </div>
        </div>
      </Modal>
    </div>
  );
};
