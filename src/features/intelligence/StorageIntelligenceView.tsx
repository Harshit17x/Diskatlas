import React, { useEffect, useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type { StorageConsumer } from '../../types/bindings';
import { useIntelligenceStore } from '../../stores/intelligenceStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  Brain,
  ShieldCheck,
  ShieldAlert,
  AlertTriangle,
  RotateCw,
  Search,
  Sparkles,
  ChevronDown,
  ChevronRight,
  HardDrive,
  Info,
  CheckCircle2,
  FolderLock,
  ArrowRight,
  ChevronLeft,
} from 'lucide-react';

export const StorageIntelligenceView: React.FC = () => {
  const {
    profile,
    isLoadingProfile,
    recommendations,
    isLoadingRecommendations,
    query,
    setQuery,
    fetchProfile,
    fetchRecommendations,
    selectedSessionId,
    setSelectedSessionId,
  } = useIntelligenceStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();
  const [expandedRecId, setExpandedRecId] = useState<string | null>(null);

  useEffect(() => {
    fetchSessions(1, 20);
    fetchProfile();
  }, [fetchSessions, fetchProfile]);

  const handleSessionChange = (sid: string) => {
    setSelectedSessionId(sid || null);
    fetchProfile(sid || undefined);
  };

  const handleGroupFilter = (group: string) => {
    setQuery({ group, page: 1 });
    fetchRecommendations();
  };

  const handleRiskFilter = (riskLevel: string) => {
    setQuery({ riskLevel, page: 1 });
    fetchRecommendations();
  };

  const handleSearch = (search: string) => {
    setQuery({ search, page: 1 });
    fetchRecommendations();
  };

  const handlePageChange = (newPage: number) => {
    setQuery({ page: newPage });
    fetchRecommendations();
  };

  const getGroupBadge = (group: string) => {
    switch (group) {
      case 'safe_to_review':
        return <Badge variant="success" size="sm">Safe to Review</Badge>;
      case 'review_required':
        return <Badge variant="warning" size="sm">Review Required</Badge>;
      case 'system_protected':
        return <Badge variant="danger" size="sm">System Protected</Badge>;
      case 'move_or_archive':
        return <Badge variant="accent" size="sm">Move or Archive</Badge>;
      case 'uninstall_official':
        return <Badge variant="neutral" size="sm">Official Uninstall</Badge>;
      default:
        return <Badge variant="neutral" size="sm">{group}</Badge>;
    }
  };

  const getRiskBadge = (risk: string) => {
    switch (risk) {
      case 'none':
      case 'low':
        return <Badge variant="success" size="sm">Low Risk</Badge>;
      case 'moderate':
        return <Badge variant="warning" size="sm">Moderate Risk</Badge>;
      case 'high':
        return <Badge variant="danger" size="sm">High Risk</Badge>;
      case 'critical':
        return <Badge variant="danger" size="sm">Critical (Do Not Delete)</Badge>;
      default:
        return <Badge variant="neutral" size="sm">{risk}</Badge>;
    }
  };

  const getConfidenceBadge = (confidence: number) => {
    const pct = Math.round(confidence * 100);
    if (confidence >= 0.95) {
      return <Badge variant="success" size="sm">{pct}% Evidence</Badge>;
    }
    if (confidence >= 0.8) {
      return <Badge variant="accent" size="sm">{pct}% Evidence</Badge>;
    }
    return <Badge variant="neutral" size="sm">{pct}% Heuristic</Badge>;
  };

  const consumerColumns: ColumnDef<StorageConsumer>[] = [
    {
      accessorKey: 'name',
      header: 'Consumer Name',
      cell: ({ row }) => (
        <span className="font-sans font-medium text-primary truncate block max-w-xs" title={row.original.path}>
          {row.original.name}
        </span>
      ),
    },
    {
      accessorKey: 'category',
      header: 'Category',
      cell: ({ getValue }) => <Badge variant="neutral" size="sm">{String(getValue())}</Badge>,
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Storage Footprint',
      cell: ({ getValue }) => (
        <span className="font-mono text-accent font-semibold text-xs">
          {formatBytes(Number(getValue()))}
        </span>
      ),
    },
    {
      accessorKey: 'percentageOfUsed',
      header: '% of Scanned',
      cell: ({ getValue }) => (
        <span className="font-mono text-secondary text-xs">
          {Number(getValue()).toFixed(1)}%
        </span>
      ),
    },
    {
      accessorKey: 'explanation',
      header: 'Storage Explanation',
      cell: ({ row }) => (
        <div className="max-w-md space-y-0.5">
          <span className="text-secondary text-xs block">{row.original.explanation}</span>
          <span className="font-mono text-muted text-[10px] truncate block" title={row.original.path}>
            {row.original.path}
          </span>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-4">
      {/* 1. Header & Controls */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div className="flex items-center gap-2.5">
          <div className="p-2 rounded bg-accent/10 border border-accent/30 text-accent">
            <Brain className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-base font-semibold text-primary">Storage Intelligence & Ownership Engine</h1>
            <p className="text-xs text-secondary mt-0.5">
              Deterministic explanations of disk consumption, recoverable space estimates, and safety classifications
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Target Session Selector */}
          <select
            value={selectedSessionId || ''}
            onChange={(e) => handleSessionChange(e.target.value)}
            className="bg-[#15171B] border border-border rounded px-2.5 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          >
            <option value="">Latest Completed Scan</option>
            {sessionsPage?.items.map((s) => (
              <option key={s.id} value={s.id}>
                Session {s.id.slice(0, 8)} ({s.driveId} • {formatBytes(s.totalBytes)})
              </option>
            ))}
          </select>

          <Button
            variant="secondary"
            size="sm"
            icon={<RotateCw className={`w-3.5 h-3.5 ${isLoadingProfile ? 'animate-spin' : ''}`} />}
            onClick={() => fetchProfile()}
            disabled={isLoadingProfile}
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* 2. Safety Philosophy Banner */}
      <div className="p-3 bg-[#16181C] border border-border rounded flex items-start gap-2.5 text-xs text-secondary">
        <ShieldCheck className="w-4 h-4 text-success shrink-0 mt-0.5" />
        <div className="space-y-0.5 text-[11px] leading-relaxed">
          <span className="font-semibold text-primary block">Storage Ownership Principles:</span>
          <p>
            DiskAtlas explains <em>what</em> consumes storage and <em>why</em>. Confidence scores are strictly evidence-backed.
            System-critical partitions are quarantined with zero recoverable claim. No automated cleanup or background deletion is ever executed.
          </p>
        </div>
      </div>

      {/* 3. Reclaimable Metrics Overview */}
      <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Total Scanned</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatBytes(profile?.totalScannedBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted font-mono">Volume {profile?.driveId || '—'}</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-success uppercase font-semibold flex items-center gap-1.5">
            <CheckCircle2 className="w-3.5 h-3.5 text-success" />
            Safe to Review
          </span>
          <div className="text-xl font-bold font-mono text-success mt-1">
            {formatBytes(profile?.safeReclaimableBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Low-risk redundant data</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-warning uppercase font-semibold flex items-center gap-1.5">
            <AlertTriangle className="w-3.5 h-3.5 text-warning" />
            Review Required
          </span>
          <div className="text-xl font-bold font-mono text-warning mt-1">
            {formatBytes(profile?.reviewRequiredBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">User decision needed</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-danger uppercase font-semibold flex items-center gap-1.5">
            <ShieldAlert className="w-3.5 h-3.5 text-danger" />
            System Protected
          </span>
          <div className="text-xl font-bold font-mono text-danger mt-1">
            {formatBytes(profile?.systemProtectedBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Strictly quarantined (0 B reclaimable)</span>
        </Card>
      </div>

      {/* 4. Main Macro Storage Findings */}
      {profile?.findings && profile.findings.length > 0 && (
        <div className="space-y-2">
          <h2 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
            <Sparkles className="w-3.5 h-3.5 text-accent" />
            Macro Storage Findings
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {profile.findings.map((finding) => (
              <div
                key={finding.id}
                className="p-3.5 rounded-lg border border-border bg-[#14161A] space-y-2 flex flex-col justify-between"
              >
                <div className="space-y-1">
                  <div className="flex items-center justify-between">
                    <span className="font-semibold text-primary text-xs">{finding.title}</span>
                    <Badge
                      variant={finding.severity === 'info' ? 'accent' : 'warning'}
                      size="sm"
                    >
                      {finding.severity.toUpperCase()}
                    </Badge>
                  </div>
                  <p className="text-[11px] text-secondary leading-relaxed">
                    {finding.description}
                  </p>
                </div>

                <div className="pt-2 border-t border-border/60 flex items-center justify-between text-[11px] text-muted font-mono">
                  <span>Impact: {formatBytes(finding.impactBytes)}</span>
                  <span>{finding.affectedItemsCount} items</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* 5. Largest Storage Consumers */}
      {profile?.largestConsumers && profile.largestConsumers.length > 0 && (
        <div className="space-y-2 pt-1">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
              <HardDrive className="w-3.5 h-3.5 text-secondary" />
              Largest Storage Consumers & Explanations
            </h2>
            <span className="text-[11px] text-muted">Top space allocations</span>
          </div>

          <DataTable
            columns={consumerColumns}
            data={profile.largestConsumers}
          />
        </div>
      )}

      {/* 6. Storage Recommendations Engine */}
      <div className="space-y-3 pt-3 border-t border-border">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h2 className="text-xs font-semibold text-primary uppercase tracking-wider flex items-center gap-1.5">
              <FolderLock className="w-3.5 h-3.5 text-accent" />
              Evidence-Backed Recommendations ({recommendations?.totalItems ?? 0})
            </h2>
            <p className="text-[11px] text-muted">
              Classified into 5 recommendation groups with exact path, expected consequences, and recovery estimates
            </p>
          </div>

          <div className="flex items-center gap-2">
            {/* Risk filter */}
            <select
              value={query.riskLevel || 'all'}
              onChange={(e) => handleRiskFilter(e.target.value)}
              className="bg-[#15171B] border border-border rounded px-2 py-1 text-xs text-primary font-mono focus:outline-none focus:border-accent"
            >
              <option value="all">All Risk Levels</option>
              <option value="low">Low Risk</option>
              <option value="moderate">Moderate Risk</option>
              <option value="high">High Risk</option>
              <option value="critical">Critical (System)</option>
            </select>

            {/* Search */}
            <div className="relative min-w-[200px]">
              <Search className="w-3.5 h-3.5 text-muted absolute left-2.5 top-2 pointer-events-none" />
              <input
                type="text"
                placeholder="Search path, reason..."
                value={query.search || ''}
                onChange={(e) => handleSearch(e.target.value)}
                className="w-full bg-[#15171B] border border-border rounded pl-8 pr-3 py-1 text-xs text-primary placeholder-muted focus:outline-none focus:border-accent font-mono"
              />
            </div>
          </div>
        </div>

        {/* Group Filter Chips */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1">
          {[
            { id: 'all', label: 'All Groups' },
            { id: 'safe_to_review', label: 'Safe to Review' },
            { id: 'review_required', label: 'Review Required' },
            { id: 'system_protected', label: 'System Protected' },
            { id: 'move_or_archive', label: 'Move or Archive' },
            { id: 'uninstall_official', label: 'Official Uninstall' },
          ].map((grp) => (
            <button
              key={grp.id}
              onClick={() => handleGroupFilter(grp.id)}
              className={`px-2.5 py-1 text-xs rounded transition-colors whitespace-nowrap ${
                (query.group || 'all') === grp.id
                  ? 'bg-accent text-white font-medium'
                  : 'bg-elevated text-secondary hover:text-primary hover:bg-[#282D35]'
              }`}
            >
              {grp.label}
            </button>
          ))}
        </div>

        {/* Recommendations Cards List */}
        {isLoadingRecommendations ? (
          <Card padding="lg" className="text-center py-10 space-y-2">
            <RotateCw className="w-6 h-6 text-accent animate-spin mx-auto" />
            <h3 className="text-sm font-semibold text-primary">Evaluating Storage Intelligence Rules</h3>
            <p className="text-xs text-muted max-w-sm mx-auto">
              Synthesizing recommendations against deterministic rules and risk tiers...
            </p>
          </Card>
        ) : !recommendations || recommendations.items.length === 0 ? (
          <Card padding="lg" className="text-center py-10 space-y-2">
            <CheckCircle2 className="w-8 h-8 text-success mx-auto" />
            <h3 className="text-sm font-semibold text-primary">No Recommendations for Selected Filters</h3>
            <p className="text-xs text-muted max-w-sm mx-auto">
              No files matched the current recommendation group and risk level.
            </p>
          </Card>
        ) : (
          <div className="space-y-2.5">
            {recommendations.items.map((rec) => {
              const isExpanded = expandedRecId === rec.id;
              return (
                <div
                  key={rec.id}
                  className="p-3.5 rounded-lg border border-border bg-panel hover:border-border/80 transition-colors space-y-2"
                >
                  {/* Top Bar: Name, Group, Risk, Recoverable */}
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                    <div className="flex items-center gap-2 overflow-hidden">
                      <span className="font-semibold text-primary text-xs font-mono truncate" title={rec.path}>
                        {rec.name}
                      </span>
                      {getGroupBadge(rec.group)}
                      {getRiskBadge(rec.riskLevel)}
                      {getConfidenceBadge(rec.confidence)}
                    </div>

                    <div className="flex items-center gap-4 text-xs font-mono shrink-0">
                      <div>
                        <span className="text-[10px] text-muted uppercase block">Size</span>
                        <span className="text-secondary">{formatBytes(rec.sizeBytes)}</span>
                      </div>
                      <div>
                        <span className="text-[10px] text-accent uppercase font-semibold block">Recoverable</span>
                        <span className={`font-semibold ${rec.recoverableBytes > 0 ? 'text-success' : 'text-muted'}`}>
                          {formatBytes(rec.recoverableBytes)}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Reason & Path */}
                  <div className="space-y-1 text-xs">
                    <p className="text-primary font-medium text-[11px] leading-relaxed">
                      {rec.reason}
                    </p>
                    <span className="font-mono text-muted text-[10px] truncate block" title={rec.path}>
                      {rec.path}
                    </span>
                  </div>

                  {/* Collapsible Actionable Guidance & Expected Consequences */}
                  <div className="pt-2 border-t border-border/60">
                    <button
                      onClick={() => setExpandedRecId(isExpanded ? null : rec.id)}
                      className="flex items-center gap-1.5 text-[11px] text-secondary hover:text-primary transition-colors font-medium"
                    >
                      {isExpanded ? (
                        <ChevronDown className="w-3.5 h-3.5 text-accent" />
                      ) : (
                        <ChevronRight className="w-3.5 h-3.5" />
                      )}
                      <span>Actionable Guidance & Expected Consequences</span>
                    </button>

                    {isExpanded && (
                      <div className="mt-2 p-2.5 rounded bg-[#131518] border border-border/60 space-y-2 text-[11px]">
                        <div className="space-y-0.5">
                          <span className="font-semibold text-primary flex items-center gap-1">
                            <ArrowRight className="w-3 h-3 text-accent" />
                            Recommended Action:
                          </span>
                          <p className="text-secondary pl-4">{rec.recommendedAction}</p>
                        </div>

                        <div className="space-y-0.5">
                          <span className="font-semibold text-primary flex items-center gap-1">
                            <Info className="w-3 h-3 text-warning" />
                            Expected Consequences:
                          </span>
                          <p className="text-secondary pl-4">{rec.expectedConsequences}</p>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}

            {/* Pagination Controls */}
            {recommendations.totalPages > 1 && (
              <div className="flex items-center justify-between px-2 pt-2 text-xs text-secondary border-t border-border">
                <span>
                  Page {recommendations.page} of {recommendations.totalPages} ({formatNumber(recommendations.totalItems)} total items)
                </span>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    icon={<ChevronLeft className="w-3.5 h-3.5" />}
                    onClick={() => handlePageChange(recommendations.page - 1)}
                    disabled={recommendations.page <= 1}
                  >
                    Previous
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    icon={<ChevronRight className="w-3.5 h-3.5" />}
                    onClick={() => handlePageChange(recommendations.page + 1)}
                    disabled={recommendations.page >= recommendations.totalPages}
                  >
                    Next
                  </Button>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
