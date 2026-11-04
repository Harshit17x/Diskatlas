import React, { useEffect, useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { Tabs } from '../../components/common/Tabs';
import { DataTable } from '../../components/common/DataTable';
import { Modal } from '../../components/common/Modal';
import type { ColumnDef } from '@tanstack/react-table';
import type {
  DeveloperProjectItem,
  GlobalToolCacheItem,
} from '../../types/bindings';
import { useDeveloperStore } from '../../stores/developerStore';
import { useHistoryStore } from '../../stores/historyStore';
import { formatBytes, formatNumber } from '../../utils/formatters';
import {
  Code2,
  Terminal,
  RotateCw,
  Cpu,
  Boxes,
  Eye,
  Info,
  Server,
  Package,
} from 'lucide-react';

export const DeveloperStorageView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('projects');

  const {
    profile,
    docker,
    globalCaches,
    selectedProject,
    filterEcosystem,
    isLoading,
    fetchDeveloperStorage,
    setFilterEcosystem,
    setSelectedProject,
  } = useDeveloperStore();

  const { sessionsPage, fetchSessions } = useHistoryStore();

  useEffect(() => {
    fetchSessions(1, 20);
    fetchDeveloperStorage();
  }, [fetchSessions, fetchDeveloperStorage]);

  const handleSelectSession = (sid: string) => {
    fetchDeveloperStorage(sid);
  };

  const getEcosystemBadge = (ecosystem: string) => {
    if (ecosystem.includes('Node')) return <Badge variant="accent" size="sm">Node.js</Badge>;
    if (ecosystem.includes('Rust')) return <Badge variant="warning" size="sm">Rust</Badge>;
    if (ecosystem.includes('Python')) return <Badge variant="success" size="sm">Python</Badge>;
    if (ecosystem.includes('Java')) return <Badge variant="accent" size="sm">Java</Badge>;
    if (ecosystem.includes('C/C++')) return <Badge variant="neutral" size="sm">C/C++</Badge>;
    if (ecosystem.includes('Multi-stack')) return <Badge variant="accent" size="sm">{ecosystem}</Badge>;
    return <Badge variant="neutral" size="sm">Git Repo</Badge>;
  };

  const formatTimestamp = (ts: number | null) => {
    if (!ts) return '—';
    try {
      return new Date(ts * 1000).toLocaleDateString(undefined, {
        dateStyle: 'short',
      });
    } catch {
      return '—';
    }
  };

  const filteredProjects = profile?.projects.filter((p) => {
    if (!filterEcosystem || filterEcosystem === 'ALL') return true;
    return p.ecosystem.toLowerCase().includes(filterEcosystem.toLowerCase());
  }) || [];

  const projectColumns: ColumnDef<DeveloperProjectItem>[] = [
    {
      accessorKey: 'name',
      header: 'Project Name',
      cell: ({ row }) => (
        <div className="flex items-center gap-2 max-w-xs">
          <Code2 className="w-4 h-4 text-accent shrink-0" />
          <div className="truncate">
            <span className="font-semibold text-primary truncate block">{row.original.name}</span>
            <span className="text-[11px] text-muted font-mono truncate block" title={row.original.rootPath}>
              {row.original.rootPath}
            </span>
          </div>
        </div>
      ),
    },
    {
      accessorKey: 'ecosystem',
      header: 'Technology',
      cell: ({ getValue }) => getEcosystemBadge(String(getValue())),
    },
    {
      accessorKey: 'dependencyBytes',
      header: 'Dependencies',
      cell: ({ getValue }) => {
        const val = Number(getValue());
        return val > 0 ? (
          <span className="font-mono text-warning font-medium text-xs">{formatBytes(val)}</span>
        ) : (
          <span className="text-muted text-xs font-mono">—</span>
        );
      },
    },
    {
      accessorKey: 'cacheBytes',
      header: 'Caches',
      cell: ({ getValue }) => {
        const val = Number(getValue());
        return val > 0 ? (
          <span className="font-mono text-secondary text-xs">{formatBytes(val)}</span>
        ) : (
          <span className="text-muted text-xs font-mono">—</span>
        );
      },
    },
    {
      accessorKey: 'buildOutputBytes',
      header: 'Build Output',
      cell: ({ getValue }) => {
        const val = Number(getValue());
        return val > 0 ? (
          <span className="font-mono text-danger font-medium text-xs">{formatBytes(val)}</span>
        ) : (
          <span className="text-muted text-xs font-mono">—</span>
        );
      },
    },
    {
      accessorKey: 'totalBytes',
      header: 'Total Project Size',
      cell: ({ getValue }) => (
        <span className="font-mono text-primary font-bold text-xs">{formatBytes(Number(getValue()))}</span>
      ),
    },
    {
      accessorKey: 'lastModifiedAt',
      header: 'Last Active',
      cell: ({ getValue }) => (
        <span className="text-muted text-xs font-mono">
          {formatTimestamp(getValue() as number | null)}
        </span>
      ),
    },
    {
      accessorKey: 'confidence',
      header: 'Confidence',
      cell: ({ getValue }) => (
        <Badge variant={Number(getValue()) >= 0.9 ? 'success' : 'neutral'} size="sm">
          {(Number(getValue()) * 100).toFixed(0)}%
        </Badge>
      ),
    },
    {
      id: 'actions',
      header: '',
      cell: ({ row }) => (
        <button
          onClick={() => setSelectedProject(row.original)}
          className="p-1 rounded text-secondary hover:text-primary hover:bg-elevated transition-colors"
          title="Inspect project breakdown"
        >
          <Eye className="w-3.5 h-3.5" />
        </button>
      ),
    },
  ];

  const globalCacheColumns: ColumnDef<GlobalToolCacheItem>[] = [
    {
      accessorKey: 'tool',
      header: 'Package Manager / Tooling',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <Package className="w-3.5 h-3.5 text-accent" />
          <span className="font-medium text-primary">{row.original.tool}</span>
        </div>
      ),
    },
    {
      accessorKey: 'path',
      header: 'Standard Host Location',
      cell: ({ getValue }) => (
        <span className="font-mono text-secondary text-xs truncate max-w-md block" title={String(getValue())}>
          {String(getValue())}
        </span>
      ),
    },
    {
      accessorKey: 'sizeBytes',
      header: 'Occupied Size',
      cell: ({ row }) =>
        row.original.exists ? (
          <span className="font-mono text-primary font-semibold text-xs">
            {formatBytes(row.original.sizeBytes)}
          </span>
        ) : (
          <span className="text-muted text-xs">Not Present</span>
        ),
    },
    {
      accessorKey: 'exists',
      header: 'Status',
      cell: ({ getValue }) =>
        getValue() ? (
          <Badge variant="success" size="sm">ACTIVE ON HOST</Badge>
        ) : (
          <Badge variant="neutral" size="sm">INACTIVE</Badge>
        ),
    },
  ];

  const totalGlobalSize = globalCaches.reduce((acc, c) => acc + c.sizeBytes, 0);

  return (
    <div className="space-y-4">
      {/* 1. Header & Quick Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-border">
        <div>
          <h1 className="text-base font-semibold text-primary">Developer Storage Intelligence</h1>
          <p className="text-xs text-secondary mt-0.5">
            Identify software project repositories, heavy node_modules, build targets, package manager stores, and Docker allocations
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Target Scan Session Selector */}
          <select
            value={profile?.sessionId || ''}
            onChange={(e) => handleSelectSession(e.target.value)}
            className="bg-[#15171B] border border-border rounded px-2.5 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          >
            {sessionsPage?.items.map((s) => (
              <option key={s.id} value={s.id}>
                Session {s.id.slice(0, 8)} ({s.driveId} • {formatBytes(s.totalBytes)})
              </option>
            ))}
          </select>

          <Button
            variant="outline"
            size="sm"
            icon={<RotateCw className="w-3.5 h-3.5" />}
            onClick={() => fetchDeveloperStorage()}
            disabled={isLoading}
            title="Refresh developer intelligence analysis"
          >
            Refresh
          </Button>
        </div>
      </div>

      {/* 2. Top Metric Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-4 gap-3">
        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Project Storage</span>
            <Code2 className="w-3.5 h-3.5 text-accent" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatBytes(profile?.totalDeveloperBytes ?? 0)}
          </div>
          <span className="text-[11px] text-muted font-mono">
            Across {profile?.totalProjects ?? 0} repositories
          </span>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Docker Engine</span>
            <Boxes className="w-3.5 h-3.5 text-accent" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {docker?.isAvailable ? formatBytes(docker.totalBytes) : 'Inactive'}
          </div>
          <span className="text-[11px] text-muted font-mono truncate block" title={docker?.statusMessage}>
            {docker?.statusMessage || 'Checking engine'}
          </span>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Global Tooling Caches</span>
            <Package className="w-3.5 h-3.5 text-warning" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {formatBytes(totalGlobalSize)}
          </div>
          <span className="text-[11px] text-muted font-mono">
            Cargo, npm, pnpm, Gradle, pip
          </span>
        </Card>

        <Card padding="md">
          <div className="flex items-center justify-between text-secondary">
            <span className="text-[11px] font-medium uppercase tracking-wider">Ecosystems</span>
            <Cpu className="w-3.5 h-3.5 text-success" />
          </div>
          <div className="text-xl font-bold font-mono text-primary mt-1">
            {profile?.ecosystems.length ?? 0} Tech Stacks
          </div>
          <span className="text-[11px] text-muted font-mono">
            Automated marker detection
          </span>
        </Card>
      </div>

      {/* 3. Ecosystem Breakdown Cards */}
      {profile && profile.ecosystems.length > 0 && (
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
          {profile.ecosystems.map((eco) => (
            <div
              key={eco.ecosystem}
              onClick={() => setFilterEcosystem(filterEcosystem === eco.ecosystem ? null : eco.ecosystem)}
              className={`p-2.5 rounded border cursor-pointer transition-colors ${
                filterEcosystem === eco.ecosystem
                  ? 'bg-accent/15 border-accent text-accent'
                  : 'bg-elevated border-border hover:bg-elevated/80 text-secondary'
              }`}
            >
              <div className="text-[11px] font-semibold text-primary truncate">{eco.ecosystem}</div>
              <div className="text-sm font-bold font-mono text-primary mt-0.5">
                {formatBytes(eco.totalBytes)}
              </div>
              <div className="text-[10px] text-muted font-mono mt-0.5">
                {eco.projectCount} project(s)
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 4. Tabs Area */}
      <div className="space-y-3 pt-1">
        <Tabs
          tabs={[
            { id: 'projects', label: `Discovered Projects (${filteredProjects.length})` },
            { id: 'docker', label: `Docker Containers & Images (${docker?.isAvailable ? formatBytes(docker.totalBytes) : 'Offline'})` },
            { id: 'global', label: `Global Package Stores (${formatBytes(totalGlobalSize)})` },
          ]}
          activeTab={activeTab}
          onChange={setActiveTab}
        />

        {/* Tab 1: Project-wise Table */}
        {activeTab === 'projects' && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
                  Software Repositories & Artifacts
                </h2>
                {filterEcosystem && (
                  <Badge variant="accent" size="sm">
                    Filtered: {filterEcosystem}
                  </Badge>
                )}
              </div>

              {/* Ecosystem filter dropdown */}
              <div className="flex items-center gap-2 text-xs">
                <span className="text-muted text-[11px]">Filter Stack:</span>
                <select
                  value={filterEcosystem || 'ALL'}
                  onChange={(e) => setFilterEcosystem(e.target.value === 'ALL' ? null : e.target.value)}
                  className="bg-[#15171B] border border-border rounded px-2 py-1 text-xs text-primary focus:outline-none focus:border-accent"
                >
                  <option value="ALL">All Technologies</option>
                  <option value="Node.js">Node.js / JS</option>
                  <option value="Rust">Rust / Cargo</option>
                  <option value="Python">Python</option>
                  <option value="Java">Java / Android</option>
                  <option value="C/C++">C / C++</option>
                </select>
              </div>
            </div>

            <DataTable
              columns={projectColumns}
              data={filteredProjects}
              emptyMessage={
                profile
                  ? 'No developer projects discovered in the selected session. Projects are identified by package.json, Cargo.toml, pyproject.toml, pom.xml, CMakeLists.txt or .git.'
                  : 'Select a completed scan session from the dropdown above to analyze software project storage.'
              }
            />
          </div>
        )}

        {/* Tab 2: Docker Containers & Images */}
        {activeTab === 'docker' && (
          <div className="space-y-4">
            <div className="p-3 bg-elevated border border-border rounded flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <Boxes className="w-5 h-5 text-accent" />
                <div>
                  <div className="font-semibold text-xs text-primary">Docker Engine Diagnostic</div>
                  <div className="text-[11px] text-muted">{docker?.statusMessage}</div>
                </div>
              </div>
              <Badge variant={docker?.isAvailable ? 'success' : 'neutral'} size="sm">
                {docker?.isAvailable ? 'ENGINE ACTIVE' : 'INACTIVE'}
              </Badge>
            </div>

            {docker?.isAvailable ? (
              <div className="grid grid-cols-1 sm:grid-cols-4 gap-3 text-xs">
                <Card padding="md" className="space-y-1">
                  <span className="text-muted uppercase text-[10px] font-semibold">Images</span>
                  <div className="text-lg font-bold font-mono text-primary">
                    {formatBytes(docker.imagesBytes)}
                  </div>
                  <span className="text-[11px] text-secondary font-mono">
                    {docker.imagesCount} images stored
                  </span>
                </Card>

                <Card padding="md" className="space-y-1">
                  <span className="text-muted uppercase text-[10px] font-semibold">Containers Writable Layers</span>
                  <div className="text-lg font-bold font-mono text-primary">
                    {formatBytes(docker.containersBytes)}
                  </div>
                  <span className="text-[11px] text-secondary font-mono">
                    {docker.containersCount} containers
                  </span>
                </Card>

                <Card padding="md" className="space-y-1">
                  <span className="text-muted uppercase text-[10px] font-semibold">Local Volumes</span>
                  <div className="text-lg font-bold font-mono text-warning">
                    {formatBytes(docker.volumesBytes)}
                  </div>
                  <span className="text-[11px] text-secondary font-mono">
                    {docker.volumesCount} persistent volumes
                  </span>
                </Card>

                <Card padding="md" className="space-y-1">
                  <span className="text-muted uppercase text-[10px] font-semibold">Build Cache</span>
                  <div className="text-lg font-bold font-mono text-accent">
                    {formatBytes(docker.buildCacheBytes)}
                  </div>
                  <span className="text-[11px] text-secondary font-mono">
                    BuildKit layer caches
                  </span>
                </Card>
              </div>
            ) : (
              <Card padding="lg" className="text-center py-8 space-y-2">
                <Server className="w-8 h-8 text-muted mx-auto" />
                <h3 className="text-sm font-semibold text-primary">Docker Engine Not Reachable</h3>
                <p className="text-xs text-muted max-w-sm mx-auto">
                  Start Docker Desktop to query live image, container, volume, and build-cache consumption metrics via supported engine APIs.
                </p>
              </Card>
            )}

            <div className="p-3 bg-[#131518] border border-border rounded flex items-start gap-2.5 text-xs text-muted">
              <Info className="w-4 h-4 text-accent shrink-0 mt-0.5" />
              <p className="text-[11px]">
                <strong className="text-secondary">Safety Guarantee:</strong> DiskAtlas queries Docker allocation exclusively via official Docker engine APIs. DiskAtlas never directly tampers with or manipulates raw Docker storage files on disk.
              </p>
            </div>
          </div>
        )}

        {/* Tab 3: Global Package Manager & Tooling Caches */}
        {activeTab === 'global' && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-xs font-semibold text-primary uppercase tracking-wider">
                  Global Tooling & Package Manager Caches
                </h2>
                <p className="text-[11px] text-muted">
                  Package registries, compiler outputs, and emulator stores stored in user home directories.
                </p>
              </div>
              <Badge variant="accent" size="sm">
                Total: {formatBytes(totalGlobalSize)}
              </Badge>
            </div>

            <DataTable
              columns={globalCacheColumns}
              data={globalCaches}
              emptyMessage="No global tool caches detected on this system."
            />
          </div>
        )}
      </div>

      {/* 5. Project Breakdown Modal */}
      {selectedProject && (
        <Modal
          isOpen={true}
          onClose={() => setSelectedProject(null)}
          title={`Project: ${selectedProject.name}`}
          description={selectedProject.rootPath}
          maxWidth="lg"
          footer={
            <Button variant="ghost" onClick={() => setSelectedProject(null)}>
              Close
            </Button>
          }
        >
          <div className="space-y-4 text-xs">
            {/* Top stats */}
            <div className="grid grid-cols-3 gap-2">
              <div className="p-2.5 bg-elevated border border-border rounded">
                <span className="text-muted block text-[10px] uppercase">Ecosystem</span>
                <span className="font-semibold text-primary text-sm mt-0.5 block">
                  {selectedProject.ecosystem}
                </span>
              </div>
              <div className="p-2.5 bg-elevated border border-border rounded">
                <span className="text-muted block text-[10px] uppercase">Total Footprint</span>
                <span className="font-mono font-bold text-primary text-sm mt-0.5 block">
                  {formatBytes(selectedProject.totalBytes)}
                </span>
              </div>
              <div className="p-2.5 bg-elevated border border-border rounded">
                <span className="text-muted block text-[10px] uppercase">Files Discovered</span>
                <span className="font-mono font-semibold text-primary text-sm mt-0.5 block">
                  {formatNumber(selectedProject.fileCount)}
                </span>
              </div>
            </div>

            {/* Storage roles distribution */}
            <div className="space-y-2">
              <h3 className="text-xs font-semibold text-primary uppercase tracking-wider">
                Artifact Roles Breakdown
              </h3>

              <div className="p-3 bg-[#14161A] border border-border rounded space-y-2">
                <div className="flex justify-between items-center">
                  <span className="text-secondary flex items-center gap-1.5">
                    <Package className="w-3.5 h-3.5 text-warning" />
                    Dependencies (node_modules, venv, Pods)
                  </span>
                  <span className="font-mono text-warning font-semibold">
                    {formatBytes(selectedProject.dependencyBytes)}
                  </span>
                </div>

                <div className="flex justify-between items-center">
                  <span className="text-secondary flex items-center gap-1.5">
                    <Terminal className="w-3.5 h-3.5 text-accent" />
                    Caches (.next/cache, .vite, __pycache__, turbo)
                  </span>
                  <span className="font-mono text-secondary font-medium">
                    {formatBytes(selectedProject.cacheBytes)}
                  </span>
                </div>

                <div className="flex justify-between items-center">
                  <span className="text-secondary flex items-center gap-1.5">
                    <Boxes className="w-3.5 h-3.5 text-danger" />
                    Build Outputs (target, dist, build, out)
                  </span>
                  <span className="font-mono text-danger font-semibold">
                    {formatBytes(selectedProject.buildOutputBytes)}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
};
