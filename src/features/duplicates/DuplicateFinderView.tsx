import React, { useEffect, useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { useAdvancedAnalysisStore } from '../../stores/advancedAnalysisStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  Copy,
  Play,
  RotateCw,
  ShieldCheck,
  ChevronDown,
  ChevronRight,
  Hash,
  FileCheck2,
  Clock,
} from 'lucide-react';

export const DuplicateFinderView: React.FC = () => {
  const {
    duplicateResult,
    duplicatesLoading,
    duplicateQuery,
    setDuplicateQuery,
    searchDuplicates,
    selectedSessionId,
    setSelectedSessionId,
  } = useAdvancedAnalysisStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();
  const [expandedHashes, setExpandedHashes] = useState<Record<string, boolean>>({});

  useEffect(() => {
    fetchSessions(1, 20);
    searchDuplicates();
  }, [fetchSessions, searchDuplicates]);

  const toggleExpand = (hash: string) => {
    setExpandedHashes((prev) => ({
      ...prev,
      [hash]: !prev[hash],
    }));
  };

  const handleSessionChange = (sid: string) => {
    setSelectedSessionId(sid || null);
    searchDuplicates(sid || undefined);
  };

  const handleMinSizePreset = (bytes: number) => {
    setDuplicateQuery({ minSizeBytes: bytes });
  };

  return (
    <div className="space-y-4">
      {/* 1. Header & Controls */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Staged Duplicate Detection</h1>
          <p className="text-xs text-secondary mt-0.5">
            Byte-for-byte cryptographic SHA-256 verification through multi-stage candidate filtering
          </p>
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
            variant="primary"
            size="sm"
            icon={<Play className={`w-3.5 h-3.5 ${duplicatesLoading ? 'animate-spin' : ''}`} />}
            onClick={() => searchDuplicates()}
            disabled={duplicatesLoading}
          >
            {duplicatesLoading ? 'Analyzing...' : 'Search Duplicates'}
          </Button>

          <Button
            variant="secondary"
            size="sm"
            icon={<RotateCw className="w-3.5 h-3.5" />}
            onClick={() => searchDuplicates()}
            disabled={duplicatesLoading}
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* 2. Engineering & Methodology Principles */}
      <div className="p-3 bg-[#16181C] border border-border rounded flex items-start gap-2.5 text-xs text-secondary">
        <ShieldCheck className="w-4 h-4 text-accent shrink-0 mt-0.5" />
        <div className="space-y-1 text-[11px] leading-relaxed">
          <span className="font-semibold text-primary block">Staged Verification Methodology:</span>
          <p>
            Equal filenames are <strong className="text-primary font-semibold">never</strong> treated as proof of duplication.
            DiskAtlas uses a 4-stage pipeline: (1) Size-bucket partitioning, (2) Metadata evaluation, (3) 64KB streaming SHA-256 cryptographic hashing, and (4) Hash-cluster grouping.
            Only exact binary clones are flagged. Visually similar pictures with altered compression or metadata are strictly distinguished from identical files.
          </p>
        </div>
      </div>

      {/* 3. Top Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Duplicate Groups</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatNumber(duplicateResult?.groupsCount ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Unique content clusters</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-warning uppercase font-semibold">Wasted Storage</span>
          <div className="text-xl font-bold font-mono text-warning mt-1">
            {formatBytes(duplicateResult?.totalWastedBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Redundant duplicate copies</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-accent uppercase font-semibold">Total Redundant Files</span>
          <div className="text-xl font-bold font-mono text-accent mt-1">
            {formatNumber(duplicateResult?.totalDuplicateFiles ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Individual duplicate instances</span>
        </Card>

        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Verification Engine</span>
          <div className="text-xs font-semibold text-primary mt-2">
            {duplicateResult?.hashingAlgorithm || 'SHA-256 (Staged)'}
          </div>
          <span className="text-[11px] text-success">Bit-level exact match</span>
        </Card>
      </div>

      {/* 4. Filter Toolbar */}
      <Card padding="md" className="space-y-2">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-2">
            <span className="text-muted font-medium">Candidate Size Threshold:</span>
            {[
              { label: '≥ 1 MB', bytes: 1048576 },
              { label: '≥ 10 MB', bytes: 10485760 },
              { label: '≥ 50 MB', bytes: 52428800 },
              { label: '≥ 100 MB', bytes: 104857600 },
            ].map((p) => (
              <button
                key={p.label}
                onClick={() => handleMinSizePreset(p.bytes)}
                className={`px-2 py-0.5 rounded text-[11px] font-mono transition-colors ${
                  duplicateQuery.minSizeBytes === p.bytes
                    ? 'bg-accent text-white font-semibold'
                    : 'bg-elevated text-secondary hover:text-primary hover:bg-[#282D35]'
                }`}
              >
                {p.label}
              </button>
            ))}
          </div>

          <div className="flex items-center gap-2">
            <span className="text-muted font-medium">Extension Filter:</span>
            <input
              type="text"
              placeholder="e.g. mp4, zip, iso..."
              value={duplicateQuery.extension || ''}
              onChange={(e) => setDuplicateQuery({ extension: e.target.value })}
              className="bg-[#15171B] border border-border rounded px-2 py-0.5 text-xs text-primary font-mono placeholder-muted focus:outline-none focus:border-accent w-32"
            />
          </div>
        </div>
      </Card>

      {/* 5. Duplicate Clusters List */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
            Identified Binary Duplicate Sets ({duplicateResult?.groups.length ?? 0})
          </h2>
          <span className="text-[11px] text-muted">
            Sorted by potential storage reclaim
          </span>
        </div>

        {duplicatesLoading ? (
          <Card padding="lg" className="text-center py-12 space-y-2">
            <RotateCw className="w-6 h-6 text-accent animate-spin mx-auto" />
            <h3 className="text-sm font-semibold text-primary">Running Staged Duplicate Analysis</h3>
            <p className="text-xs text-muted max-w-sm mx-auto">
              Comparing size partitions and computing cryptographic SHA-256 hashes on matching candidates...
            </p>
          </Card>
        ) : !duplicateResult || duplicateResult.groups.length === 0 ? (
          <Card padding="lg" className="text-center py-12 space-y-2">
            <FileCheck2 className="w-8 h-8 text-success mx-auto" />
            <h3 className="text-sm font-semibold text-primary">No Duplicate Files Detected</h3>
            <p className="text-xs text-muted max-w-sm mx-auto">
              All indexed files within the selected size threshold have unique binary signatures or no copies were found.
            </p>
          </Card>
        ) : (
          <div className="space-y-2">
            {duplicateResult.groups.map((group) => {
              const isExpanded = expandedHashes[group.hash] ?? false;
              return (
                <div
                  key={group.hash}
                  className="border border-border rounded-lg bg-panel overflow-hidden transition-colors"
                >
                  {/* Group Header */}
                  <div
                    onClick={() => toggleExpand(group.hash)}
                    className="p-3.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3 cursor-pointer hover:bg-elevated/60 transition-colors"
                  >
                    <div className="flex items-start sm:items-center gap-3">
                      <button className="text-muted hover:text-primary mt-0.5 sm:mt-0">
                        {isExpanded ? (
                          <ChevronDown className="w-4 h-4 text-accent" />
                        ) : (
                          <ChevronRight className="w-4 h-4" />
                        )}
                      </button>

                      <div className="p-2 rounded bg-elevated border border-border">
                        <Copy className="w-4 h-4 text-warning" />
                      </div>

                      <div className="space-y-0.5">
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-primary text-xs font-mono">
                            {group.sampleName}
                          </span>
                          <Badge variant="warning" size="sm">
                            {group.copiesCount} identical copies
                          </Badge>
                        </div>
                        <div className="flex items-center gap-2 text-[11px] text-muted font-mono">
                          <Hash className="w-3 h-3 text-muted" />
                          <span>SHA-256: {group.hash.slice(0, 16)}...</span>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-4 text-right pl-7 sm:pl-0">
                      <div>
                        <span className="text-[10px] text-muted block uppercase font-medium">Unit Size</span>
                        <span className="font-mono text-secondary text-xs">
                          {formatBytes(group.unitSizeBytes)}
                        </span>
                      </div>
                      <div>
                        <span className="text-[10px] text-warning block uppercase font-semibold">Wasted Reclaimable</span>
                        <span className="font-mono text-warning font-semibold text-xs">
                          {formatBytes(group.wastedBytes)}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Expanded Duplicate File Paths */}
                  {isExpanded && (
                    <div className="border-t border-border bg-[#131518] p-3 space-y-2">
                      <div className="text-[11px] font-semibold text-secondary px-1 flex items-center justify-between">
                        <span>Physical File Locations ({group.files.length})</span>
                        <span className="text-[10px] text-muted italic">Non-destructive view • No deletion executed</span>
                      </div>

                      <div className="space-y-1.5">
                        {group.files.map((file, idx) => (
                          <div
                            key={file.id || idx}
                            className="p-2 rounded bg-elevated/40 border border-border/60 flex items-center justify-between gap-3 text-xs"
                          >
                            <div className="flex items-center gap-2 overflow-hidden">
                              <span className="text-[10px] font-mono text-muted w-5 shrink-0">#{idx + 1}</span>
                              <span className="font-mono text-primary text-[11px] truncate" title={file.path}>
                                {file.path}
                              </span>
                            </div>

                            <div className="flex items-center gap-3 shrink-0 text-muted font-mono text-[10px]">
                              {file.modifiedAt && (
                                <div className="flex items-center gap-1">
                                  <Clock className="w-3 h-3" />
                                  <span>{new Date(file.modifiedAt * 1000).toLocaleDateString()}</span>
                                </div>
                              )}
                              <span>{formatBytes(file.sizeBytes)}</span>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
