import React, { useState } from 'react';
import { Card } from '../../components/common/Card';
import { Button } from '../../components/common/Button';
import { Tabs } from '../../components/common/Tabs';
import { useDriveStore } from '../../stores/driveStore';
import {
  Database,
  Cpu,
  Keyboard,
  FolderMinus,
} from 'lucide-react';

export const SettingsView: React.FC = () => {
  const [activeTab, setActiveTab] = useState('general');
  const { dbHealth, systemInfo } = useDriveStore();

  const [excludedDirs, setExcludedDirs] = useState<string[]>([
    'C:\\System Volume Information',
    'C:\\$Recycle.Bin',
    '/System/Volumes/Data/.Spotlight-V100',
    '**/.git/**',
  ]);

  const [newExclusion, setNewExclusion] = useState('');

  const handleAddExclusion = (e: React.FormEvent) => {
    e.preventDefault();
    if (newExclusion.trim() && !excludedDirs.includes(newExclusion.trim())) {
      setExcludedDirs([...excludedDirs, newExclusion.trim()]);
      setNewExclusion('');
    }
  };

  const handleRemoveExclusion = (path: string) => {
    setExcludedDirs(excludedDirs.filter((p) => p !== path));
  };

  return (
    <div className="space-y-4 max-w-4xl">
      {/* Header */}
      <div className="pb-3 border-b border-border">
        <h1 className="text-base font-semibold text-primary">Settings & Preferences</h1>
        <p className="text-xs text-secondary mt-0.5">
          Configure scanning boundaries, database maintenance, exclusions, and shortcuts
        </p>
      </div>

      <Tabs
        tabs={[
          { id: 'general', label: 'General & Exclusions' },
          { id: 'database', label: 'SQLite Database' },
          { id: 'shortcuts', label: 'Keyboard Shortcuts' },
        ]}
        activeTab={activeTab}
        onChange={setActiveTab}
      />

      {activeTab === 'general' && (
        <div className="space-y-4">
          {/* Exclusion Rules */}
          <Card padding="md" className="space-y-3">
            <div className="flex items-center gap-2 pb-2 border-b border-border">
              <FolderMinus className="w-4 h-4 text-warning" />
              <h2 className="text-xs font-semibold text-primary">Filesystem Exclusions</h2>
            </div>
            <p className="text-xs text-secondary">
              Patterns and paths that the scanner will completely bypass during traversal:
            </p>

            <form onSubmit={handleAddExclusion} className="flex gap-2">
              <input
                type="text"
                value={newExclusion}
                onChange={(e) => setNewExclusion(e.target.value)}
                placeholder="e.g. C:\Users\Username\Private or **/node_modules/**"
                className="flex-1 bg-[#14161A] border border-border rounded px-3 py-1.5 text-xs text-primary placeholder-muted focus:outline-none focus:border-accent"
              />
              <Button type="submit" variant="primary" size="sm">
                Add Exclusion
              </Button>
            </form>

            <div className="space-y-1.5 pt-2">
              {excludedDirs.map((dir) => (
                <div
                  key={dir}
                  className="flex items-center justify-between px-3 py-2 bg-[#15171B] border border-border rounded text-xs font-mono"
                >
                  <span className="text-primary truncate">{dir}</span>
                  <button
                    onClick={() => handleRemoveExclusion(dir)}
                    className="text-muted hover:text-danger text-xs font-sans ml-2"
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          </Card>

          {/* Engine Parameters */}
          <Card padding="md" className="space-y-3">
            <div className="flex items-center gap-2 pb-2 border-b border-border">
              <Cpu className="w-4 h-4 text-accent" />
              <h2 className="text-xs font-semibold text-primary">Scanner Engine Configuration</h2>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
              <div className="p-3 bg-[#15171B] border border-border rounded space-y-1">
                <span className="font-semibold text-primary block">Parallel Threads</span>
                <p className="text-secondary text-[11px]">
                  Configured to use Rayon thread pool with {systemInfo?.cpuCount || 'host'} cores.
                </p>
              </div>

              <div className="p-3 bg-[#15171B] border border-border rounded space-y-1">
                <span className="font-semibold text-primary block">Symlink Loop Protection</span>
                <p className="text-secondary text-[11px]">
                  Always enabled. Directory symlinks & NTFS junctions are never recursed.
                </p>
              </div>
            </div>
          </Card>
        </div>
      )}

      {activeTab === 'database' && (
        <Card padding="md" className="space-y-4">
          <div className="flex items-center gap-2 pb-2 border-b border-border">
            <Database className="w-4 h-4 text-success" />
            <h2 className="text-xs font-semibold text-primary">SQLite Diagnostics & Storage</h2>
          </div>

          <div className="space-y-2 text-xs font-mono">
            <div className="p-3 bg-[#14161A] border border-border rounded flex flex-col gap-1">
              <span className="text-muted uppercase text-[10px] font-sans">Database Location</span>
              <span className="text-primary break-all">{dbHealth?.dbPath || 'Detecting...'}</span>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <div className="p-2.5 bg-[#14161A] border border-border rounded">
                <span className="text-muted uppercase text-[10px] font-sans block">Schema Version</span>
                <span className="text-primary font-semibold text-sm">v{dbHealth?.migrationVersion || 1}</span>
              </div>
              <div className="p-2.5 bg-[#14161A] border border-border rounded">
                <span className="text-muted uppercase text-[10px] font-sans block">Journal Mode</span>
                <span className="text-success font-semibold text-sm">WAL (Write-Ahead)</span>
              </div>
            </div>
          </div>

          <div className="pt-2 flex items-center gap-2">
            <Button variant="secondary" size="sm">
              Verify Database Integrity
            </Button>
            <Button variant="secondary" size="sm">
              VACUUM Database
            </Button>
          </div>
        </Card>
      )}

      {activeTab === 'shortcuts' && (
        <Card padding="md" className="space-y-3">
          <div className="flex items-center gap-2 pb-2 border-b border-border">
            <Keyboard className="w-4 h-4 text-accent" />
            <h2 className="text-xs font-semibold text-primary">Desktop Keyboard Shortcuts</h2>
          </div>

          <div className="divide-y divide-border text-xs">
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-secondary">Open Command Palette / Quick Search</span>
              <kbd className="px-2 py-0.5 font-mono text-[11px] rounded bg-[#14161A] border border-border text-primary">
                ⌘K / Ctrl+K
              </kbd>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-secondary">Close Dialog or Modal</span>
              <kbd className="px-2 py-0.5 font-mono text-[11px] rounded bg-[#14161A] border border-border text-primary">
                Escape
              </kbd>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-secondary">Go to Dashboard</span>
              <kbd className="px-2 py-0.5 font-mono text-[11px] rounded bg-[#14161A] border border-border text-primary">
                ⌘1
              </kbd>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-secondary">Go to Drive Explorer</span>
              <kbd className="px-2 py-0.5 font-mono text-[11px] rounded bg-[#14161A] border border-border text-primary">
                ⌘2
              </kbd>
            </div>
            <div className="py-2.5 flex items-center justify-between">
              <span className="text-secondary">Refresh Discovered Volumes</span>
              <kbd className="px-2 py-0.5 font-mono text-[11px] rounded bg-[#14161A] border border-border text-primary">
                ⌘R
              </kbd>
            </div>
          </div>
        </Card>
      )}
    </div>
  );
};
