import React, { useEffect, useMemo, useState, useCallback } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import type { ColumnDef } from '@tanstack/react-table';
import type {
  AppLeftoverItem,
  AppStatItem,
  ApplicationsInventoryReport,
  CleanupItemCandidate,
  InstalledAppInfo,
} from '../../types/bindings';
import { safeInvoke } from '../../utils/tauri';
import { useAnalysisStore } from '../../stores/analysisStore';
import { useCleanupStore } from '../../stores/cleanupStore';
import { useHistoryStore } from '../../stores/historyStore';
import { CleanupConfirmationModal } from '../../components/cleanup/CleanupConfirmationModal';
import { CleanupResultsBanner } from '../../components/cleanup/CleanupResultsBanner';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  Play,
  RotateCw,
  AppWindow,
  Trash2,
  Search,
  Sparkles,
  PackageX,
  Lock,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';

export const ApplicationsView: React.FC = () => {
  const { activeProfile, isLoading, fetchAnalysis, runClassification } = useAnalysisStore();
  const { sessionsPage, fetchSessions } = useHistoryStore();
  const { openDirectCleanupConfirmation } = useCleanupStore();

  const [inventory, setInventory] = useState<ApplicationsInventoryReport | null>(null);
  const [loadingInventory, setLoadingInventory] = useState(false);
  const [inventoryError, setInventoryError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState('installed');
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedAppId, setExpandedAppId] = useState<string | null>(null);
  const [selectedLeftoverIds, setSelectedLeftoverIds] = useState<Set<string>>(new Set());

  const fetchInventory = useCallback(async () => {
    setLoadingInventory(true);
    setInventoryError(null);
    const res = await safeInvoke<ApplicationsInventoryReport>('inspect_installed_applications');
    if (res.ok) {
      setInventory(res.data);
      setSelectedLeftoverIds(new Set());
    } else {
      setInventoryError(res.error.message);
    }
    setLoadingInventory(false);
  }, []);

  useEffect(() => {
    fetchSessions(1, 20);
    fetchAnalysis();
    fetchInventory();
  }, [fetchSessions, fetchAnalysis, fetchInventory]);

  // Classified software footprint fallback from SQLite scan
  const classifiedApps: AppStatItem[] = useMemo(() => {
    if (!activeProfile?.categories) return [];
    const map = new Map<string, AppStatItem>();
    for (const cat of activeProfile.categories) {
      for (const app of cat.topApps) {
        const existing = map.get(app.appName);
        if (existing) {
          existing.fileCount += app.fileCount;
          existing.totalBytes += app.totalBytes;
        } else {
          map.set(app.appName, { ...app });
        }
      }
    }
    return Array.from(map.values()).sort((a, b) => b.totalBytes - a.totalBytes);
  }, [activeProfile]);

  const filteredApps = useMemo(() => {
    if (!inventory) return [];
    const q = searchQuery.trim().toLowerCase();
    if (!q) return inventory.apps;
    return inventory.apps.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        (a.bundleId && a.bundleId.toLowerCase().includes(q)) ||
        a.appPath.toLowerCase().includes(q)
    );
  }, [inventory, searchQuery]);

  const filteredLeftovers = useMemo(() => {
    if (!inventory) return [];
    const q = searchQuery.trim().toLowerCase();
    if (!q) return inventory.leftovers;
    return inventory.leftovers.filter(
      (l) =>
        l.appHint.toLowerCase().includes(q) ||
        l.path.toLowerCase().includes(q) ||
        (l.bundleId && l.bundleId.toLowerCase().includes(q))
    );
  }, [inventory, searchQuery]);

  const handleAnalyze = () => {
    if (activeProfile?.sessionId) {
      runClassification(activeProfile.sessionId);
    } else if (sessionsPage?.items[0]?.id) {
      runClassification(sessionsPage.items[0].id);
    }
  };

  const askCleanAppCaches = (app: InstalledAppInfo) => {
    const safeCaches = app.associatedPaths.filter((p) => p.isCacheSafe && p.sizeBytes > 0);
    if (safeCaches.length === 0) return;
    const candidates: CleanupItemCandidate[] = safeCaches.map((p, idx) => ({
      id: `${app.id}-cache-${idx}`,
      path: p.path,
      name: `${app.name} (${p.kind.replace('_', ' ')})`,
      sizeBytes: p.sizeBytes,
      category: 'temporary',
      group: 'safe_to_review',
      riskLevel: 'low',
      reason: `Regenerable cache/state files for ${app.name}.`,
      confidence: 0.95,
      recoverableBytes: p.sizeBytes,
      expectedConsequences: `${app.name} recreates temporary caches automatically; app and preferences stay intact.`,
      recommendedAction: 'Move app cache files to Trash.',
      isCloudSynced: false,
      cloudWarning: null,
      isBlocked: false,
      blockReason: null,
      selected: true,
      existsOnDisk: true,
    }));

    openDirectCleanupConfirmation({
      title: `Clean Caches for ${app.name}`,
      items: candidates,
      source: 'applications',
      onSuccess: fetchInventory,
    });
  };

  const askUninstallApp = (app: InstalledAppInfo) => {
    if (app.isSystemApp) return;
    const candidates: CleanupItemCandidate[] = [
      {
        id: `${app.id}-bundle`,
        path: app.appPath,
        name: `${app.name}.app`,
        sizeBytes: app.bundleSizeBytes,
        category: 'applications',
        group: 'review_required',
        riskLevel: 'moderate',
        reason: `Application bundle${app.bundleId ? ` (${app.bundleId})` : ''}.`,
        confidence: 0.95,
        recoverableBytes: app.bundleSizeBytes,
        expectedConsequences: 'Moves application bundle to Trash (restorable from Trash if needed).',
        recommendedAction: 'Quit the application before moving to Trash.',
        isCloudSynced: false,
        cloudWarning: null,
        isBlocked: false,
        blockReason: null,
        selected: true,
        existsOnDisk: true,
      },
      ...app.associatedPaths.map((p, idx) => ({
        id: `${app.id}-assoc-${idx}`,
        path: p.path,
        name: `${app.name} • ${p.kind.replace('_', ' ')}`,
        sizeBytes: p.sizeBytes,
        category: p.isCacheSafe ? 'temporary' : 'applications',
        group: 'review_required' as const,
        riskLevel: (p.isCacheSafe ? 'low' : 'moderate') as 'low' | 'moderate',
        reason: `Associated ${p.kind.replace('_', ' ')} data discovered in ~/Library.`,
        confidence: 0.9,
        recoverableBytes: p.sizeBytes,
        expectedConsequences: 'Moves app support/cache/preference remnants cleanly to Trash.',
        recommendedAction: 'Review associated Library paths before confirming cleanup.',
        isCloudSynced: false,
        cloudWarning: null,
        isBlocked: false,
        blockReason: null,
        selected: true,
        existsOnDisk: true,
      })),
    ];

    openDirectCleanupConfirmation({
      title: `Move ${app.name} & Leftovers to Trash`,
      items: candidates,
      source: 'applications',
      onSuccess: fetchInventory,
    });
  };

  const toggleLeftover = (id: string) => {
    const next = new Set(selectedLeftoverIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setSelectedLeftoverIds(next);
  };

  const askCleanSelectedLeftovers = () => {
    if (!inventory) return;
    const selected = inventory.leftovers.filter((l) => selectedLeftoverIds.has(l.id));
    if (selected.length === 0) return;

    const candidates: CleanupItemCandidate[] = selected.map((l: AppLeftoverItem) => ({
      id: l.id,
      path: l.path,
      name: `${l.appHint} (${l.kind.replace('_', ' ')})`,
      sizeBytes: l.sizeBytes,
      category: 'temporary',
      group: l.isCacheSafe ? 'safe_to_review' : 'review_required',
      riskLevel: l.isCacheSafe ? 'low' : 'moderate',
      reason: l.reason,
      confidence: 0.9,
      recoverableBytes: l.sizeBytes,
      expectedConsequences: 'Moves orphaned Library remnant to Trash cleanly.',
      recommendedAction: 'Confirm app is no longer installed before removing leftovers.',
      isCloudSynced: false,
      cloudWarning: null,
      isBlocked: false,
      blockReason: null,
      selected: true,
      existsOnDisk: true,
    }));

    openDirectCleanupConfirmation({
      title: 'Clean Orphaned App Leftovers',
      items: candidates,
      source: 'applications',
      onSuccess: fetchInventory,
    });
  };

  const classifiedColumns: ColumnDef<AppStatItem>[] = [
    {
      accessorKey: 'appName',
      header: 'Application / Suite',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <AppWindow className="w-4 h-4 text-accent flex-shrink-0" />
          <span className="font-semibold text-primary">{row.original.appName}</span>
        </div>
      ),
    },
    {
      accessorKey: 'fileCount',
      header: 'Indexed Components',
      cell: ({ getValue }) => (
        <span className="text-secondary font-mono text-xs">{formatNumber(Number(getValue()))} files</span>
      ),
    },
    {
      accessorKey: 'totalBytes',
      header: 'Storage Footprint',
      cell: ({ getValue }) => (
        <span className="text-accent font-mono font-semibold text-xs">{formatBytes(Number(getValue()))}</span>
      ),
    },
  ];

  const selectedLeftoverBytes = useMemo(() => {
    if (!inventory) return 0;
    return inventory.leftovers
      .filter((l) => selectedLeftoverIds.has(l.id))
      .reduce((acc, l) => acc + l.sizeBytes, 0);
  }, [inventory, selectedLeftoverIds]);

  return (
    <div className="space-y-4">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Applications &amp; Leftovers Inspector</h1>
          <p className="text-xs text-secondary mt-0.5">
            Inspect installed applications, app Library caches/containers, and orphaned leftovers — always asks permission before Trash removal
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            icon={<RotateCw className={`w-3.5 h-3.5 ${loadingInventory ? 'animate-spin' : ''}`} />}
            onClick={fetchInventory}
            disabled={loadingInventory}
          >
            {loadingInventory ? 'Scanning Apps...' : 'Rescan Apps'}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            icon={<Play className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
            onClick={handleAnalyze}
            disabled={isLoading}
          >
            {isLoading ? 'Classifying...' : 'Classify Scan Apps'}
          </Button>
        </div>
      </div>

      <CleanupResultsBanner />

      {inventoryError && (
        <Card className="p-3 border-[#DB6262]/40 bg-[#DB6262]/10 text-xs text-[#DB6262]">
          {inventoryError}
        </Card>
      )}

      {/* Summary Metric Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Installed User Apps</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {inventory ? inventory.apps.length : classifiedApps.length}
          </div>
          <span className="text-[11px] text-muted">
            {inventory ? `Scanned in ${(inventory.scanDurationMillis / 1000).toFixed(2)}s` : 'Bundles discovered'}
          </span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-accent uppercase font-semibold">App Bundles Size</span>
          <div className="text-xl font-bold font-mono text-accent mt-1">
            {formatBytes(inventory?.totalAppsBytes ?? classifiedApps.reduce((a, b) => a + b.totalBytes, 0))}
          </div>
          <span className="text-[11px] text-muted">.app binaries &amp; frameworks</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-muted uppercase font-semibold">Library Support &amp; Caches</span>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatBytes(inventory?.totalSupportBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">Caches, containers &amp; support</span>
        </Card>
        <Card padding="md">
          <span className="text-[11px] text-warning uppercase font-semibold">Orphaned Leftovers</span>
          <div className="text-xl font-bold font-mono text-warning mt-1">
            {formatBytes(inventory?.totalLeftoversBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted">
            {inventory?.leftovers.length ?? 0} uninstalled app remnants
          </span>
        </Card>
      </div>

      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <Tabs
          tabs={[
            { id: 'installed', label: 'Installed Applications', count: inventory?.apps.length },
            { id: 'leftovers', label: 'Orphaned Leftovers', count: inventory?.leftovers.length },
            { id: 'classified', label: 'Scan Classification Suites', count: classifiedApps.length },
          ]}
          activeTab={activeTab}
          onChange={setActiveTab}
        />

        {(activeTab === 'installed' || activeTab === 'leftovers') && (
          <div className="relative w-full sm:w-64">
            <Search className="w-3.5 h-3.5 text-muted absolute left-2.5 top-2.5" />
            <input
              type="text"
              placeholder="Filter apps or bundle ID..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full bg-panel border border-border rounded pl-8 pr-3 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
            />
          </div>
        )}
      </div>

      {/* TAB 1: INSTALLED APPS & ASSOCIATED LIBRARY FOOTPRINT */}
      {activeTab === 'installed' && (
        <div className="space-y-2">
          {loadingInventory && !inventory ? (
            <Card className="p-8 text-center text-xs text-secondary">
              Measuring installed application bundles and Library leftovers in parallel...
            </Card>
          ) : filteredApps.length === 0 ? (
            <Card className="p-8 text-center text-xs text-secondary">
              No installed user application bundles matched your filter.
            </Card>
          ) : (
            filteredApps.map((app) => {
              const isExpanded = expandedAppId === app.id;
              return (
                <Card key={app.id} className="p-3.5 space-y-2 border border-border">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <button
                      type="button"
                      onClick={() => setExpandedAppId(isExpanded ? null : app.id)}
                      className="flex items-center gap-2.5 text-left min-w-0 flex-1"
                    >
                      {isExpanded ? (
                        <ChevronDown className="w-4 h-4 text-muted shrink-0" />
                      ) : (
                        <ChevronRight className="w-4 h-4 text-muted shrink-0" />
                      )}
                      <AppWindow className="w-4 h-4 text-accent shrink-0" />
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="font-semibold text-xs text-primary truncate">{app.name}</span>
                          {app.version && (
                            <Badge variant="neutral" size="sm">
                              v{app.version}
                            </Badge>
                          )}
                          {app.isSystemApp && (
                            <Badge variant="warning" size="sm" icon={<Lock className="w-3 h-3" />}>
                              System App
                            </Badge>
                          )}
                        </div>
                        <div className="text-[11px] text-muted font-mono truncate">
                          {app.bundleId || app.appPath}
                        </div>
                      </div>
                    </button>

                    <div className="flex flex-wrap items-center gap-2 shrink-0">
                      <div className="text-right mr-2">
                        <div className="font-mono text-xs font-semibold text-primary">
                          {formatBytes(app.bundleSizeBytes + app.supportSizeBytes)}
                        </div>
                        <div className="text-[10px] text-muted font-mono">
                          App {formatBytes(app.bundleSizeBytes)} • Data {formatBytes(app.supportSizeBytes)}
                        </div>
                      </div>

                      {app.cacheSizeBytes > 0 && (
                        <Button
                          size="sm"
                          variant="outline"
                          icon={<Sparkles className="w-3 h-3 text-success" />}
                          onClick={() => askCleanAppCaches(app)}
                        >
                          Clean Caches ({formatBytes(app.cacheSizeBytes)})
                        </Button>
                      )}

                      <Button
                        size="sm"
                        variant="danger"
                        disabled={app.isSystemApp}
                        icon={app.isSystemApp ? <Lock className="w-3 h-3" /> : <Trash2 className="w-3 h-3" />}
                        onClick={() => askUninstallApp(app)}
                      >
                        {app.isSystemApp ? 'Protected' : 'Trash App'}
                      </Button>
                    </div>
                  </div>

                  {isExpanded && (
                    <div className="pt-2 mt-2 border-t border-border/50 space-y-1.5 text-[11px]">
                      <div className="flex justify-between font-mono text-secondary px-2 py-1 bg-elevated/60 rounded">
                        <span className="truncate" title={app.appPath}>
                          [bundle] {app.appPath}
                        </span>
                        <span className="font-semibold text-primary ml-2 shrink-0">
                          {formatBytes(app.bundleSizeBytes)}
                        </span>
                      </div>
                      {app.associatedPaths.length === 0 ? (
                        <div className="text-muted px-2 py-1 italic">
                          No associated Library support or cache paths detected.
                        </div>
                      ) : (
                        app.associatedPaths.map((p, idx) => (
                          <div
                            key={idx}
                            className="flex justify-between items-center font-mono text-secondary px-2 py-1 bg-elevated/40 rounded"
                          >
                            <span className="truncate flex items-center gap-1.5" title={p.path}>
                              <Badge variant={p.isCacheSafe ? 'success' : 'neutral'} size="sm">
                                {p.kind}
                              </Badge>
                              <span className="truncate">{p.path}</span>
                            </span>
                            <span className="font-semibold text-accent ml-2 shrink-0">
                              {formatBytes(p.sizeBytes)}
                            </span>
                          </div>
                        ))
                      )}
                    </div>
                  )}
                </Card>
              );
            })
          )}
        </div>
      )}

      {/* TAB 2: ORPHANED LEFTOVERS FROM UNINSTALLED APPS */}
      {activeTab === 'leftovers' && (
        <div className="space-y-3">
          {filteredLeftovers.length === 0 ? (
            <Card className="p-8 text-center text-xs text-secondary space-y-2">
              <PackageX className="w-6 h-6 text-muted mx-auto" />
              <div>No orphaned Library leftovers found for uninstalled apps.</div>
            </Card>
          ) : (
            <>
              <div className="space-y-2">
                {filteredLeftovers.map((item) => {
                  const isSel = selectedLeftoverIds.has(item.id);
                  return (
                    <Card
                      key={item.id}
                      className={`p-3 border transition-colors ${
                        isSel ? 'border-accent/40 bg-elevated/80' : 'border-border bg-panel'
                      }`}
                    >
                      <div className="flex items-start gap-3">
                        <input
                          type="checkbox"
                          checked={isSel}
                          onChange={() => toggleLeftover(item.id)}
                          className="w-4 h-4 mt-0.5 rounded border-border text-accent cursor-pointer"
                        />
                        <div className="flex-1 min-w-0 space-y-1">
                          <div className="flex items-center justify-between gap-2">
                            <div className="flex items-center gap-2 truncate">
                              <span className="font-semibold text-xs text-primary truncate">
                                {item.appHint}
                              </span>
                              <Badge variant={item.isCacheSafe ? 'success' : 'warning'} size="sm">
                                {item.kind.replace('_', ' ')}
                              </Badge>
                            </div>
                            <span className="font-mono text-xs font-semibold text-accent shrink-0">
                              {formatBytes(item.sizeBytes)}
                            </span>
                          </div>
                          <div className="font-mono text-[11px] text-secondary truncate" title={item.path}>
                            {item.path}
                          </div>
                          <div className="text-[11px] text-muted">{item.reason}</div>
                        </div>
                      </div>
                    </Card>
                  );
                })}
              </div>

              <div className="sticky bottom-0 p-3 bg-panel/95 backdrop-blur-md border border-border rounded-lg flex items-center justify-between z-20 px-4 shadow-lg">
                <div className="text-xs text-secondary">
                  Selected Leftovers:{' '}
                  <strong className="text-primary font-mono">{selectedLeftoverIds.size}</strong> (
                  <strong className="text-accent font-mono">{formatBytes(selectedLeftoverBytes)}</strong>)
                </div>
                <Button
                  variant="danger"
                  size="sm"
                  disabled={selectedLeftoverIds.size === 0}
                  icon={<Trash2 className="w-3.5 h-3.5" />}
                  onClick={askCleanSelectedLeftovers}
                >
                  Ask Permission to Clean Selected ({selectedLeftoverIds.size})
                </Button>
              </div>
            </>
          )}
        </div>
      )}

      {/* TAB 3: EXISTING CLASSIFIED SOFTWARE SUITES TABLE */}
      {activeTab === 'classified' && (
        <div className="space-y-2">
          <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
            Indexed Scan Software Registry
          </h2>
          <DataTable
            columns={classifiedColumns}
            data={classifiedApps}
            emptyMessage="No classified application inventory from a full scan yet. Click 'Classify Scan Apps' or use Installed Applications above."
          />
        </div>
      )}

      <CleanupConfirmationModal />
    </div>
  );
};
