import React, { useEffect, useState, useMemo } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type { ForgottenFileItem, CleanupAuditRecord, CleanupItemCandidate } from '../../types/bindings';
import { useCleanupStore } from '../../stores/cleanupStore';
import { useAdvancedAnalysisStore } from '../../stores/advancedAnalysisStore';
import { useHistoryStore } from '../../stores/historyStore';
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
} from 'lucide-react';

export const CleanupCenterView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('plan');

  const {
    cleanupPlan,
    loadingPlan,
    fetchCleanupPlan,
    selectedIds,
    toggleCandidateSelection,
    selectAllSafe,
    deselectAll,
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
    fetchSessions(1, 20);
    fetchAuditHistory(1);
    fetchForgottenFiles();
  }, [fetchSessions, fetchAuditHistory, fetchForgottenFiles]);

  // Load cleanup plan when active session is determined or changed
  useEffect(() => {
    const sid = selectedSessionId || (sessionsPage?.items[0]?.id ?? null);
    if (sid) {
      fetchCleanupPlan(sid);
    }
  }, [selectedSessionId, sessionsPage, fetchCleanupPlan]);

  const handleSessionChange = (sid: string) => {
    setSelectedSessionId(sid || null);
    if (sid) {
      fetchCleanupPlan(sid);
      fetchForgottenFiles(sid);
    }
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
    <div className="space-y-4 pb-12">
      {/* 1. Header & Session Selector */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Safe Cleanup Center</h1>
          <p className="text-xs text-secondary mt-0.5">
            Transparent, user-controlled storage cleanup with Windows Recycle Bin protection and audit history
          </p>
        </div>

        <div className="flex items-center gap-2">
          <select
            value={selectedSessionId || ''}
            onChange={(e) => handleSessionChange(e.target.value)}
            className="bg-[#15171B] border border-border rounded px-2.5 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          >
            {sessionsPage?.items.map((session) => (
              <option key={session.id} value={session.id}>
                Session {session.id.slice(0, 8)} ({session.driveId} - {session.status})
              </option>
            ))}
          </select>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              if (selectedSessionId) fetchCleanupPlan(selectedSessionId);
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
          { id: 'plan', label: 'Cleanup Plan & Recommendations', count: cleanupPlan?.totalItems },
          { id: 'audit', label: 'Cleanup Audit History', count: auditTotalItems },
          { id: 'forgotten', label: 'Forgotten Files Heuristics', count: forgottenFiles?.totalItems },
        ]}
        activeTab={activeTab}
        onChange={setActiveTab}
      />

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
                  className="w-full bg-[#101114] border border-border rounded pl-8 pr-3 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
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
                className="bg-[#101114] border border-border rounded px-2 py-0.5 text-xs text-primary font-mono"
              >
                <option value="all">All Risks</option>
                <option value="low">Low Risk</option>
                <option value="moderate">Moderate Risk</option>
                <option value="high">High Risk</option>
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
                        ? 'opacity-65 border-border bg-[#101114]/50'
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
          <div className="fixed bottom-0 left-64 right-0 p-3 bg-panel/95 backdrop-blur-md border-t border-border flex items-center justify-between z-30 px-6">
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
                  fetchForgottenFiles();
                }}
              >
                &gt; 30 Days
              </Button>
              <Button
                variant={forgottenFilesQuery.minAgeDays === 60 ? 'primary' : 'outline'}
                size="sm"
                onClick={() => {
                  setForgottenFilesQuery({ minAgeDays: 60, page: 1 });
                  fetchForgottenFiles();
                }}
              >
                &gt; 60 Days
              </Button>
              <Button
                variant={forgottenFilesQuery.minAgeDays === 90 ? 'primary' : 'outline'}
                size="sm"
                onClick={() => {
                  setForgottenFilesQuery({ minAgeDays: 90, page: 1 });
                  fetchForgottenFiles();
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
        </Card>
      )}

      {/* Confirmation Modal */}
      <CleanupConfirmationModal />
    </div>
  );
};
