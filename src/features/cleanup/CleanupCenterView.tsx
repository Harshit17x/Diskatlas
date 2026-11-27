import React, { useEffect, useState, useMemo } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type {
  ForgottenFileItem,
  CleanupAuditRecord,
  CleanupItemCandidate,
  QuickSweepItem,
} from '../../types/bindings';
import { useCleanupStore } from '../../stores/cleanupStore';
import { useAdvancedAnalysisStore } from '../../stores/advancedAnalysisStore';
import { useHistoryStore } from '../../stores/historyStore';
import { useUIStore } from '../../stores/uiStore';
import { formatBytes } from '../../utils/formatters';
import { CleanupConfirmationModal } from '../../components/cleanup/CleanupConfirmationModal';
import { CleanupResultsBanner } from '../../components/cleanup/CleanupResultsBanner';
import {
  CheckCircle2,
  RotateCw,
  Clock,
  Archive,
  Download,
  Terminal,
  FileCode,
  Package,
  ChevronLeft,
  ChevronRight,
  Info,
  Trash2,
  ShieldAlert,
  Search,
  CloudOff,
  Filter,
  CheckSquare,
  Square,
  Lock,
  Zap,
} from 'lucide-react';

export const CleanupCenterView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('quick_sweep');
  const [sweepCategoryFilter, setSweepCategoryFilter] = useState<string>('all');
  const [sweepSearch, setSweepSearch] = useState('');
  const { requestedCleanupTab, setRequestedCleanupTab } = useUIStore();

  const {
    cleanupPlan,
    loadingPlan,
    fetchCleanupPlan,
    selectedIds,
    toggleCandidateSelection,
    selectAllSafe,
    deselectAll,
    quickSweepReport,
    loadingQuickSweep,
    quickSweepSelectedIds,
    runQuickSweep,
    toggleQuickSweepSelection,
    selectAllSafeQuickSweep,
    deselectAllQuickSweep,
    openDirectCleanupConfirmation,
    groupFilter,
    setGroupFilter,
    riskFilter,
    setRiskFilter,
    searchQuery,
    setSearchQuery,
    setConfirmationModalOpen,
    auditRecords,
    auditTotalItems,
    auditPage,
    auditTotalPages,
    fetchAuditHistory,
  } = useCleanupStore();

  const {
    forgottenFiles,
    forgottenFilesQuery,
    setForgottenFilesQuery,
    fetchForgottenFiles,
    selectedSessionId,
    setSelectedSessionId,
  } = useAdvancedAnalysisStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();

  useEffect(() => {
    if (requestedCleanupTab) {
      setActiveTab(requestedCleanupTab);
      setRequestedCleanupTab(null);
    }
  }, [requestedCleanupTab, setRequestedCleanupTab]);

  useEffect(() => {
    fetchSessions(1, 20);
    fetchAuditHistory(1);
    fetchForgottenFiles();
    fetchCleanupPlan(selectedSessionId);
    if (!quickSweepReport && !loadingQuickSweep) {
      runQuickSweep();
    }
  }, [
    fetchSessions,
    fetchAuditHistory,
    fetchForgottenFiles,
    fetchCleanupPlan,
    selectedSessionId,
    quickSweepReport,
    loadingQuickSweep,
    runQuickSweep,
  ]);

  const handleSessionChange = (sid: string) => {
    const nextSid = sid || null;
    setSelectedSessionId(nextSid);
    fetchCleanupPlan(nextSid);
    fetchForgottenFiles(nextSid || undefined);
  };

  // Filter candidates based on group, risk, and search query
  const filteredCandidates = useMemo(() => {
    if (!cleanupPlan) return [];
    return cleanupPlan.candidates.filter((c) => {
      // Group filter
      if (groupFilter === 'safe_to_review' && c.group !== 'safe_to_review') return false;
      if (groupFilter === 'review_required' && c.group !== 'review_required') return false;
      if (groupFilter === 'system_protected' && !c.isBlocked) return false;

      // Risk filter
      if (riskFilter !== 'all' && c.riskLevel !== riskFilter) return false;

      // Search query
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        return (
          c.name.toLowerCase().includes(q) ||
          c.path.toLowerCase().includes(q) ||
          c.category.toLowerCase().includes(q) ||
          c.reason.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [cleanupPlan, groupFilter, riskFilter, searchQuery]);

  // Selected candidates stats
  const selectedCandidates = useMemo(() => {
    if (!cleanupPlan) return [];
    return cleanupPlan.candidates.filter((c) => selectedIds.has(c.id));
  }, [cleanupPlan, selectedIds]);

  const selectedBytes = useMemo(() => {
    return selectedCandidates.reduce((acc, c) => acc + c.sizeBytes, 0);
  }, [selectedCandidates]);

  // Forgotten files heuristics table columns
  const getSignalBadge = (signal: string) => {
    switch (signal) {
      case 'old_download':
        return (
          <Badge variant="accent" size="sm" icon={<Download className="w-3 h-3" />}>
            Old Download
          </Badge>
        );
      case 'old_installer':
        return (
          <Badge variant="warning" size="sm" icon={<Package className="w-3 h-3" />}>
            Old Installer
          </Badge>
        );
      case 'old_archive':
        return (
          <Badge variant="neutral" size="sm" icon={<Archive className="w-3 h-3" />}>
            Old Archive
          </Badge>
        );
      case 'old_backup':
        return (
          <Badge variant="neutral" size="sm" icon={<FileCode className="w-3 h-3" />}>
            Old Backup
          </Badge>
        );
      case 'stale_build_artifact':
        return (
          <Badge variant="warning" size="sm" icon={<Terminal className="w-3 h-3" />}>
            Stale Build Output
          </Badge>
        );
      default:
        return <Badge variant="neutral" size="sm">{signal}</Badge>;
    }
  };

  const forgottenColumns: ColumnDef<ForgottenFileItem>[] = [
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
      accessorKey: 'signalType',
      header: 'Heuristic Signal',
      cell: ({ row }) => getSignalBadge(row.original.signalType),
    },
    {
      accessorKey: 'ageDays',
      header: 'Untouched Age',
      cell: ({ getValue }) => (
        <div className="flex items-center gap-1.5 text-secondary font-mono text-xs">
          <Clock className="w-3.5 h-3.5 text-muted shrink-0" />
          <span>{Number(getValue())} days</span>
        </div>
      ),
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Size',
      cell: ({ getValue }) => (
        <span className="font-mono text-accent font-semibold text-xs">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'path',
      header: 'Location & Advisory Note',
      cell: ({ row }) => (
        <div className="space-y-0.5 max-w-md">
          <span className="font-mono text-secondary text-[11px] truncate block" title={row.original.path}>
            {row.original.path}
          </span>
          <span className="text-[10px] text-muted block truncate italic" title={row.original.safetyHeuristic}>
            {row.original.safetyHeuristic}
          </span>
        </div>
      ),
    },
  ];

  const filteredQuickSweepItems = useMemo(() => {
    if (!quickSweepReport) return [];
    return quickSweepReport.items.filter((item) => {
      if (sweepCategoryFilter !== 'all' && item.sweepGroup !== sweepCategoryFilter) return false;
      if (sweepSearch.trim()) {
        const q = sweepSearch.toLowerCase();
        return (
          item.name.toLowerCase().includes(q) ||
          item.path.toLowerCase().includes(q) ||
          item.groupLabel.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [quickSweepReport, sweepCategoryFilter, sweepSearch]);

  const selectedQuickSweepItems = useMemo(() => {
    if (!quickSweepReport) return [];
    return quickSweepReport.items.filter((i) => quickSweepSelectedIds.has(i.id));
  }, [quickSweepReport, quickSweepSelectedIds]);

  const selectedQuickSweepBytes = useMemo(() => {
    return selectedQuickSweepItems.reduce((acc, i) => acc + i.sizeBytes, 0);
  }, [selectedQuickSweepItems]);

  const handleConfirmQuickSweepCleanup = () => {
    if (selectedQuickSweepItems.length === 0) return;
    const candidates: CleanupItemCandidate[] = selectedQuickSweepItems.map((item: QuickSweepItem) => ({
      id: item.id,
      path: item.path,
      name: item.name,
      sizeBytes: item.sizeBytes,
      category: item.category,
      group: item.recGroup,
      riskLevel: item.riskLevel,
      reason: item.reason,
      confidence: 0.95,
      recoverableBytes: item.sizeBytes,
      expectedConsequences: item.expectedConsequences,
      recommendedAction: item.recommendedAction,
      isCloudSynced: item.isCloudSynced,
      cloudWarning: item.cloudWarning,
      isBlocked: item.isBlocked,
      blockReason: item.blockReason,
      selected: true,
      existsOnDisk: item.existsOnDisk,
    }));
    openDirectCleanupConfirmation({
      title: 'Confirm Quick Sweep Cleanup',
      sessionId: selectedSessionId,
      items: candidates,
      source: 'quick_sweep',
    });
  };

  // Audit table columns
  const auditColumns: ColumnDef<CleanupAuditRecord>[] = [
    {
      accessorKey: 'createdAt',
      header: 'Timestamp',
      cell: ({ getValue }) => (
        <span className="font-mono text-secondary text-xs">{String(getValue())}</span>
      ),
    },
    {
      accessorKey: 'name',
      header: 'Item & Location',
      cell: ({ row }) => (
        <div className="space-y-0.5 max-w-sm">
          <span className="font-semibold text-primary text-xs block truncate">{row.original.name}</span>
          <span className="font-mono text-[10px] text-muted block truncate" title={row.original.path}>
            {row.original.path}
          </span>
        </div>
      ),
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Reclaimed',
      cell: ({ getValue }) => (
        <span className="font-mono text-accent text-xs font-semibold">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'cleanupMethod',
      header: 'Method',
      cell: ({ getValue }) => (
        <Badge variant={getValue() === 'recycle_bin' ? 'success' : 'neutral'} size="sm">
          {getValue() === 'recycle_bin' ? 'Recycle Bin' : 'Permanent'}
        </Badge>
      ),
    },
    {
      accessorKey: 'status',
      header: 'Status',
      cell: ({ row }) => {
        const s = row.original.status;
        if (s === 'success') {
          return (
            <Badge variant="success" size="sm" icon={<CheckCircle2 className="w-3 h-3 text-success" />}>
              Success
            </Badge>
          );
        }
        if (s === 'blocked_security') {
          return (
            <Badge variant="warning" size="sm" icon={<ShieldAlert className="w-3 h-3 text-warning" />}>
              Security Blocked
            </Badge>
          );
        }
        if (s === 'permission_denied') {
          return (
            <Badge variant="danger" size="sm" icon={<Lock className="w-3 h-3 text-[#DB6262]" />}>
              Permission Denied
            </Badge>
          );
        }
        return (
          <Badge variant="neutral" size="sm">
            {s.replace('_', ' ')}
          </Badge>
        );
      },
    },
    {
      accessorKey: 'isReversible',
      header: 'Reversible',
      cell: ({ getValue }) => (
        <span className={`text-[11px] font-mono ${getValue() ? 'text-success' : 'text-muted'}`}>
          {getValue() ? 'Yes (Recycle Bin)' : 'No'}
        </span>
      ),
    },
  ];

  const getRiskBadge = (risk: string) => {
    switch (risk) {
      case 'none':
      case 'low':
        return <Badge variant="success" size="sm">Low Risk</Badge>;
      case 'moderate':
        return <Badge variant="warning" size="sm">Moderate Risk</Badge>;
      case 'high':
      case 'critical':
        return <Badge variant="danger" size="sm">High Risk</Badge>;
      default:
        return <Badge variant="neutral" size="sm">{risk}</Badge>;
    }
  };

  return (
    <div className="space-y-4">
      {/* 1. Header & Session Selector */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Safe Cleanup Center</h1>
          <p className="text-xs text-secondary mt-0.5">
            Transparent, user-controlled storage cleanup with Recycle Bin / Trash protection and audit history
          </p>
        </div>

        <div className="flex items-center gap-2">
          <select
            value={selectedSessionId || ''}
            onChange={(e) => handleSessionChange(e.target.value)}
            className="bg-panel border border-border rounded px-2.5 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          >
            <option value="">Latest Completed Scan</option>
            {sessionsPage?.items.map((session) => (
              <option key={session.id} value={session.id}>
                Session {session.id.slice(0, 8)} ({session.driveId} - {session.status})
              </option>
            ))}
          </select>

          <Button
            variant="primary"
            size="sm"
            onClick={() => {
              setActiveTab('quick_sweep');
              runQuickSweep();
            }}
            disabled={loadingQuickSweep}
            icon={<Zap className={`w-3.5 h-3.5 ${loadingQuickSweep ? 'animate-pulse' : ''}`} />}
          >
            {loadingQuickSweep ? 'Sweeping...' : 'Quick Sweep'}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              fetchCleanupPlan(selectedSessionId);
              fetchAuditHistory(1);
            }}
            icon={<RotateCw className="w-3.5 h-3.5" />}
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* Execution Results Banner */}
      <CleanupResultsBanner />

      {/* 2. Primary Tabs */}
      <Tabs
        tabs={[
          { id: 'quick_sweep', label: 'Quick Sweep (Fast Caches)', count: quickSweepReport?.totalItems },
          { id: 'plan', label: 'Scan Cleanup Plan', count: cleanupPlan?.totalItems },
          { id: 'audit', label: 'Audit History', count: auditTotalItems },
          { id: 'forgotten', label: 'Forgotten Files', count: forgottenFiles?.totalItems },
        ]}
        activeTab={activeTab}
        onChange={setActiveTab}
      />

      {/* TAB 0: MOLE-STYLE FAST QUICK SWEEP (Read-only discovery; asks permission before delete) */}
      {activeTab === 'quick_sweep' && (
        <div className="space-y-4">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <Card className="p-3">
              <span className="text-[11px] text-secondary uppercase font-mono">Reclaimable Found</span>
              <div className="text-base font-bold text-primary mt-1">
                {quickSweepReport ? formatBytes(quickSweepReport.totalBytes) : '—'}
              </div>
              <div className="text-[11px] text-muted font-mono mt-0.5">
                {quickSweepReport ? `Swept in ${(quickSweepReport.durationMillis / 1000).toFixed(2)}s` : 'Read-only scan'}
              </div>
            </Card>
            <Card className="p-3">
              <span className="text-[11px] text-secondary uppercase font-mono">Safe Caches &amp; Logs</span>
              <div className="text-base font-bold text-success mt-1">
                {quickSweepReport ? formatBytes(quickSweepReport.safeBytes) : '—'}
              </div>
              <div className="text-[11px] text-muted mt-0.5">Regenerable caches</div>
            </Card>
            <Card className="p-3">
              <span className="text-[11px] text-secondary uppercase font-mono">Review Suggested</span>
              <div className="text-base font-bold text-warning mt-1">
                {quickSweepReport ? formatBytes(quickSweepReport.reviewBytes) : '—'}
              </div>
              <div className="text-[11px] text-muted mt-0.5">Installers, leftovers &amp; repos</div>
            </Card>
            <Card className="p-3 border-accent/40 bg-accent/5">
              <span className="text-[11px] text-accent uppercase font-mono">Selected to Trash</span>
              <div className="text-base font-bold text-accent mt-1">
                {selectedQuickSweepItems.length} items
              </div>
              <div className="text-[11px] text-accent/80 font-mono mt-0.5">
                {formatBytes(selectedQuickSweepBytes)}
              </div>
            </Card>
          </div>

          <Card className="p-3 space-y-3">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <div className="relative flex-1 max-w-sm">
                <Search className="w-3.5 h-3.5 text-muted absolute left-2.5 top-2.5" />
                <input
                  type="text"
                  placeholder="Filter caches, build outputs, downloads..."
                  value={sweepSearch}
                  onChange={(e) => setSweepSearch(e.target.value)}
                  className="w-full bg-elevated border border-border rounded pl-8 pr-3 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
                />
              </div>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={selectAllSafeQuickSweep}
                  icon={<CheckSquare className="w-3.5 h-3.5 text-success" />}
                >
                  Select All Safe
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={deselectAllQuickSweep}
                  icon={<Square className="w-3.5 h-3.5" />}
                >
                  Deselect All
                </Button>
              </div>
            </div>

            {quickSweepReport && (
              <div className="flex flex-wrap items-center gap-1.5 pt-2 border-t border-border/40 text-xs">
                <button
                  type="button"
                  onClick={() => setSweepCategoryFilter('all')}
                  className={`px-2 py-0.5 rounded transition-colors ${
                    sweepCategoryFilter === 'all'
                      ? 'bg-accent text-white font-medium'
                      : 'bg-elevated text-secondary hover:text-primary'
                  }`}
                >
                  All ({quickSweepReport.totalItems})
                </button>
                {quickSweepReport.categoryTotals.map((cat) => (
                  <button
                    key={cat.sweepGroup}
                    type="button"
                    onClick={() => setSweepCategoryFilter(cat.sweepGroup)}
                    className={`px-2 py-0.5 rounded transition-colors ${
                      sweepCategoryFilter === cat.sweepGroup
                        ? 'bg-accent text-white font-medium'
                        : 'bg-elevated text-secondary hover:text-primary'
                    }`}
                  >
                    {cat.label} ({cat.itemCount} • {formatBytes(cat.totalBytes)})
                  </button>
                ))}
              </div>
            )}
          </Card>

          {loadingQuickSweep ? (
            <Card className="p-8 text-center text-secondary text-xs space-y-2">
              <Zap className="w-5 h-5 text-accent animate-pulse mx-auto" />
              <div>Running parallel cache, developer build, browser cache &amp; leftover sweep...</div>
            </Card>
          ) : filteredQuickSweepItems.length === 0 ? (
            <Card className="p-8 text-center text-secondary text-xs">
              No reclaimable cache or junk items found matching your filter.
            </Card>
          ) : (
            <div className="space-y-2">
              {filteredQuickSweepItems.map((item) => {
                const isSelected = quickSweepSelectedIds.has(item.id);
                return (
                  <Card
                    key={item.id}
                    className={`p-3.5 transition-all border ${
                      item.isBlocked
                        ? 'opacity-65 border-border bg-elevated/40'
                        : isSelected
                        ? 'border-accent/40 bg-elevated/80'
                        : 'border-border bg-panel hover:bg-elevated/40'
                    }`}
                  >
                    <div className="flex items-start gap-3">
                      <div className="pt-0.5">
                        {item.isBlocked ? (
                          <Lock className="w-4 h-4 text-muted shrink-0" />
                        ) : (
                          <input
                            type="checkbox"
                            checked={isSelected}
                            onChange={() => toggleQuickSweepSelection(item.id)}
                            className="w-4 h-4 rounded border-border text-accent focus:ring-0 focus:outline-none cursor-pointer"
                          />
                        )}
                      </div>
                      <div className="flex-1 min-w-0 space-y-1">
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div className="flex items-center gap-2 truncate">
                            <span className="font-semibold text-primary text-xs truncate">
                              {item.name}
                            </span>
                            <Badge variant="neutral" size="sm">
                              {item.groupLabel}
                            </Badge>
                            {getRiskBadge(item.riskLevel)}
                            {item.ageDays > 0 && (
                              <span className="text-[10px] font-mono text-muted">
                                {item.ageDays}d old
                              </span>
                            )}
                          </div>
                          <span className="font-mono text-xs font-semibold text-accent shrink-0">
                            {formatBytes(item.sizeBytes)}
                          </span>
                        </div>
                        <div className="font-mono text-[11px] text-secondary truncate" title={item.path}>
                          {item.path}
                        </div>
                        <div className="text-[11px] text-secondary flex items-start gap-1.5">
                          <Info className="w-3.5 h-3.5 text-muted shrink-0 mt-0.5" />
                          <span>
                            {item.reason} — <span className="text-muted">{item.expectedConsequences}</span>
                          </span>
                        </div>
                        {item.isCloudSynced && item.cloudWarning && (
                          <div className="p-2 bg-warning/10 border border-warning/20 rounded text-[11px] text-warning flex items-start gap-1.5">
                            <CloudOff className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                            <span>{item.cloudWarning}</span>
                          </div>
                        )}
                      </div>
                    </div>
                  </Card>
                );
              })}
            </div>
          )}

          <div className="sticky bottom-0 p-3 bg-panel/95 backdrop-blur-md border border-border rounded-lg flex items-center justify-between z-20 px-4 shadow-lg">
            <div className="flex items-center gap-4 text-xs">
              <span className="text-secondary">
                Selected:{' '}
                <strong className="text-primary font-mono">{selectedQuickSweepItems.length}</strong> items
              </span>
              <span className="text-secondary">
                Reclaimable:{' '}
                <strong className="text-accent font-mono">{formatBytes(selectedQuickSweepBytes)}</strong>
              </span>
            </div>
            <Button
              variant="danger"
              size="md"
              onClick={handleConfirmQuickSweepCleanup}
              disabled={selectedQuickSweepItems.length === 0}
              icon={<Trash2 className="w-4 h-4" />}
            >
              Review &amp; Ask Permission to Clean ({selectedQuickSweepItems.length})
            </Button>
          </div>
        </div>
      )}

      {/* TAB 1: CLEANUP PLAN */}
      {activeTab === 'plan' && (
        <div className="space-y-4">
          {/* Metrics summary cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <Card className="p-3">
              <span className="text-[11px] text-secondary uppercase font-mono">Safe to Review</span>
              <div className="text-base font-bold text-success mt-1">
                {cleanupPlan?.safeItems ?? 0} items
              </div>
              <div className="text-[11px] text-muted font-mono mt-0.5">
                {formatBytes(cleanupPlan?.safeBytes ?? 0)}
              </div>
            </Card>

            <Card className="p-3">
              <span className="text-[11px] text-secondary uppercase font-mono">Review Required</span>
              <div className="text-base font-bold text-warning mt-1">
                {cleanupPlan?.reviewRequiredItems ?? 0} items
              </div>
              <div className="text-[11px] text-muted font-mono mt-0.5">
                {formatBytes(cleanupPlan?.reviewRequiredBytes ?? 0)}
              </div>
            </Card>

            <Card className="p-3">
              <span className="text-[11px] text-secondary uppercase font-mono">OS Protected</span>
              <div className="text-base font-bold text-primary mt-1">
                {cleanupPlan?.blockedItems ?? 0} items
              </div>
              <div className="text-[11px] text-muted mt-0.5">Blocked by Safeguards</div>
            </Card>

            <Card className="p-3 border-accent/40 bg-accent/5">
              <span className="text-[11px] text-accent uppercase font-mono">Selected to Clean</span>
              <div className="text-base font-bold text-accent mt-1">
                {selectedCandidates.length} items
              </div>
              <div className="text-[11px] text-accent/80 font-mono mt-0.5">
                {formatBytes(selectedBytes)}
              </div>
            </Card>
          </div>

          {/* Filtering Bar */}
          <Card className="p-3 space-y-3">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              {/* Search */}
              <div className="relative flex-1 max-w-sm">
                <Search className="w-3.5 h-3.5 text-muted absolute left-2.5 top-2.5" />
                <input
                  type="text"
                  placeholder="Filter recommendations by path or name..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full bg-elevated border border-border rounded pl-8 pr-3 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
                />
              </div>

              {/* Quick actions */}
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={selectAllSafe}
                  icon={<CheckSquare className="w-3.5 h-3.5 text-success" />}
                >
                  Select All Safe
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={deselectAll}
                  icon={<Square className="w-3.5 h-3.5" />}
                >
                  Deselect All
                </Button>
              </div>
            </div>

            {/* Filter buttons */}
            <div className="flex flex-wrap items-center gap-2 pt-2 border-t border-border/40 text-xs">
              <span className="text-secondary text-[11px] mr-1 flex items-center gap-1 font-mono">
                <Filter className="w-3 h-3" /> Group:
              </span>
              <button
                onClick={() => setGroupFilter('all')}
                className={`px-2 py-0.5 rounded text-xs transition-colors ${
                  groupFilter === 'all' ? 'bg-accent text-white' : 'bg-elevated text-secondary hover:text-primary'
                }`}
              >
                All ({cleanupPlan?.totalItems ?? 0})
              </button>
              <button
                onClick={() => setGroupFilter('safe_to_review')}
                className={`px-2 py-0.5 rounded text-xs transition-colors ${
                  groupFilter === 'safe_to_review' ? 'bg-success/20 text-success border border-success/30' : 'bg-elevated text-secondary hover:text-primary'
                }`}
              >
                Safe to Review ({cleanupPlan?.safeItems ?? 0})
              </button>
              <button
                onClick={() => setGroupFilter('review_required')}
                className={`px-2 py-0.5 rounded text-xs transition-colors ${
                  groupFilter === 'review_required' ? 'bg-warning/20 text-warning border border-warning/30' : 'bg-elevated text-secondary hover:text-primary'
                }`}
              >
                Review Required ({cleanupPlan?.reviewRequiredItems ?? 0})
              </button>
              <button
                onClick={() => setGroupFilter('system_protected')}
                className={`px-2 py-0.5 rounded text-xs transition-colors ${
                  groupFilter === 'system_protected' ? 'bg-panel text-primary border border-border' : 'bg-elevated text-secondary hover:text-primary'
                }`}
              >
                Protected ({cleanupPlan?.blockedItems ?? 0})
              </button>

              <div className="h-3 w-px bg-border mx-1" />

              <span className="text-secondary text-[11px] mr-1 font-mono">Risk:</span>
              <select
                value={riskFilter}
                onChange={(e) => setRiskFilter(e.target.value as any)}
                className="bg-elevated border border-border rounded px-2 py-0.5 text-xs text-primary font-mono"
              >
                <option value="all">All Risks</option>
                <option value="none">No Risk</option>
                <option value="low">Low Risk</option>
                <option value="moderate">Moderate Risk</option>
                <option value="high">High Risk</option>
                <option value="critical">Critical Risk</option>
              </select>
            </div>
          </Card>

          {/* Candidates List */}
          {loadingPlan ? (
            <Card className="p-8 text-center text-secondary text-xs">
              Analyzing filesystem items and constructing cleanup plan...
            </Card>
          ) : filteredCandidates.length === 0 ? (
            <Card className="p-8 text-center text-secondary text-xs">
              No cleanup candidates match the active filters.
            </Card>
          ) : (
            <div className="space-y-2.5">
              {filteredCandidates.map((candidate: CleanupItemCandidate) => {
                const isSelected = selectedIds.has(candidate.id);
                return (
                  <Card
                    key={candidate.id}
                    className={`p-3.5 transition-all border ${
                      candidate.isBlocked
                        ? 'opacity-65 border-border bg-elevated/40'
                        : isSelected
                        ? 'border-accent/40 bg-elevated/80'
                        : 'border-border bg-panel hover:bg-elevated/40'
                    }`}
                  >
                    <div className="flex items-start gap-3">
                      {/* Checkbox or Blocked Icon */}
                      <div className="pt-0.5">
                        {candidate.isBlocked ? (
                          <div title="Protected operating system component - cannot be deleted">
                            <Lock className="w-4 h-4 text-muted shrink-0" />
                          </div>
                        ) : (
                          <input
                            type="checkbox"
                            checked={isSelected}
                            onChange={() => toggleCandidateSelection(candidate.id)}
                            className="w-4 h-4 rounded border-border text-accent focus:ring-0 focus:outline-none cursor-pointer"
                          />
                        )}
                      </div>

                      {/* Details */}
                      <div className="flex-1 min-w-0 space-y-1">
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div className="flex items-center gap-2 truncate">
                            <span className="font-semibold text-primary text-xs truncate">
                              {candidate.name}
                            </span>
                            <Badge variant="neutral" size="sm">
                              {candidate.category}
                            </Badge>
                            {getRiskBadge(candidate.riskLevel)}
                          </div>
                          <div className="flex items-center gap-2 shrink-0">
                            <span className="font-mono text-xs font-semibold text-accent">
                              {formatBytes(candidate.sizeBytes)}
                            </span>
                          </div>
                        </div>

                        {/* Location */}
                        <div className="font-mono text-[11px] text-secondary truncate" title={candidate.path}>
                          {candidate.path}
                        </div>

                        {/* Rationale and consequences */}
                        <div className="text-[11px] text-secondary flex items-start gap-1.5 pt-0.5">
                          <Info className="w-3.5 h-3.5 text-muted shrink-0 mt-0.5" />
                          <span>{candidate.reason} — <span className="text-muted">{candidate.expectedConsequences}</span></span>
                        </div>

                        {/* Cloud sync warning */}
                        {candidate.isCloudSynced && candidate.cloudWarning && (
                          <div className="p-2 bg-warning/10 border border-warning/20 rounded text-[11px] text-warning flex items-start gap-1.5">
                            <CloudOff className="w-3.5 h-3.5 shrink-0 mt-0.5" />
                            <span>{candidate.cloudWarning}</span>
                          </div>
                        )}

                        {/* Blocked reason */}
                        {candidate.isBlocked && candidate.blockReason && (
                          <div className="p-2 bg-elevated/80 border border-border rounded text-[11px] text-secondary flex items-start gap-1.5">
                            <ShieldAlert className="w-3.5 h-3.5 text-warning shrink-0 mt-0.5" />
                            <span>{candidate.blockReason}</span>
                          </div>
                        )}
                      </div>
                    </div>
                  </Card>
                );
              })}
            </div>
          )}

          {/* Sticky Bottom Action Bar */}
          <div className="sticky bottom-0 p-3 bg-panel/95 backdrop-blur-md border border-border rounded-lg flex items-center justify-between z-20 px-4 shadow-lg">
            <div className="flex items-center gap-4 text-xs">
              <span className="text-secondary">
                Selected for cleanup:{' '}
                <strong className="text-primary font-mono">{selectedCandidates.length}</strong> items
              </span>
              <span className="text-secondary">
                Total recoverable space:{' '}
                <strong className="text-accent font-mono">{formatBytes(selectedBytes)}</strong>
              </span>
            </div>

            <Button
              variant="danger"
              size="md"
              onClick={() => setConfirmationModalOpen(true)}
              disabled={selectedCandidates.length === 0}
              icon={<Trash2 className="w-4 h-4" />}
            >
              Review & Execute Cleanup ({selectedCandidates.length})
            </Button>
          </div>
        </div>
      )}

      {/* TAB 2: AUDIT HISTORY */}
      {activeTab === 'audit' && (
        <Card className="p-4 space-y-3">
          <div className="flex items-center justify-between pb-2 border-b border-border">
            <div>
              <h2 className="text-xs font-semibold text-primary">Cleanup Audit Trail</h2>
              <p className="text-[11px] text-secondary mt-0.5">
                Cryptographically verifiable event log of all files deleted, moved to Recycle Bin, or blocked by safeguards
              </p>
            </div>
            <Badge variant="accent" size="sm">
              {auditTotalItems} Total Logged Actions
            </Badge>
          </div>

          <DataTable
            data={auditRecords}
            columns={auditColumns}
            emptyMessage="No cleanup operations have been performed yet."
          />

          {auditTotalPages > 1 && (
            <div className="flex items-center justify-between pt-2 border-t border-border text-xs text-secondary font-mono">
              <span>Page {auditPage} of {auditTotalPages}</span>
              <div className="flex items-center gap-1.5">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={auditPage <= 1}
                  onClick={() => fetchAuditHistory(auditPage - 1)}
                  icon={<ChevronLeft className="w-3.5 h-3.5" />}
                >
                  Previous
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={auditPage >= auditTotalPages}
                  onClick={() => fetchAuditHistory(auditPage + 1)}
                  icon={<ChevronRight className="w-3.5 h-3.5" />}
                >
                  Next
                </Button>
              </div>
            </div>
          )}
        </Card>
      )}

      {/* TAB 3: FORGOTTEN FILES HEURISTICS */}
      {activeTab === 'forgotten' && (
        <Card className="p-4 space-y-3">
          <div className="flex items-center justify-between pb-2 border-b border-border">
            <div>
              <h2 className="text-xs font-semibold text-primary">Heuristic Forgotten Files Analysis</h2>
              <p className="text-[11px] text-secondary mt-0.5">
                Stale downloads, unextracted installers, old archives, and dormant build outputs
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant={forgottenFilesQuery.minAgeDays === 30 ? 'primary' : 'outline'}
                size="sm"
                onClick={() => {
                  setForgottenFilesQuery({ minAgeDays: 30, page: 1 });
                  fetchForgottenFiles(selectedSessionId || undefined);
                }}
              >
                &gt; 30 Days
              </Button>
              <Button
                variant={forgottenFilesQuery.minAgeDays === 60 ? 'primary' : 'outline'}
                size="sm"
                onClick={() => {
                  setForgottenFilesQuery({ minAgeDays: 60, page: 1 });
                  fetchForgottenFiles(selectedSessionId || undefined);
                }}
              >
                &gt; 60 Days
              </Button>
              <Button
                variant={forgottenFilesQuery.minAgeDays === 90 ? 'primary' : 'outline'}
                size="sm"
                onClick={() => {
                  setForgottenFilesQuery({ minAgeDays: 90, page: 1 });
                  fetchForgottenFiles(selectedSessionId || undefined);
                }}
              >
                &gt; 90 Days
              </Button>
            </div>
          </div>

          <DataTable
            data={forgottenFiles?.items || []}
            columns={forgottenColumns}
            emptyMessage="No forgotten files detected for the selected threshold."
          />

          {forgottenFiles && forgottenFiles.totalPages > 1 && (
            <div className="flex items-center justify-between pt-2 border-t border-border text-xs text-secondary font-mono">
              <span>
                Page {forgottenFiles.page} of {forgottenFiles.totalPages} ({forgottenFiles.totalItems} items)
              </span>
              <div className="flex items-center gap-1.5">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={forgottenFiles.page <= 1}
                  onClick={() => {
                    setForgottenFilesQuery({ page: forgottenFiles.page - 1 });
                    fetchForgottenFiles(selectedSessionId || undefined);
                  }}
                  icon={<ChevronLeft className="w-3.5 h-3.5" />}
                >
                  Previous
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={forgottenFiles.page >= forgottenFiles.totalPages}
                  onClick={() => {
                    setForgottenFilesQuery({ page: forgottenFiles.page + 1 });
                    fetchForgottenFiles(selectedSessionId || undefined);
                  }}
                  icon={<ChevronRight className="w-3.5 h-3.5" />}
                >
                  Next
                </Button>
              </div>
            </div>
          )}
        </Card>
      )}

      {/* Confirmation Modal */}
      <CleanupConfirmationModal />
    </div>
  );
};
