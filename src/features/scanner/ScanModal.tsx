import React, { useState, useEffect } from 'react';
import {
  FolderSearch,
  HardDrive,
  Play,
  Pause,
  Square,
  AlertTriangle,
  CheckCircle2,
  Clock,
  Files,
  FolderTree,
  Database,
  RefreshCw,
  Sliders,
  FileText,
  Activity,
} from 'lucide-react';
import { Modal } from '../../components/common/Modal';
import { Button } from '../../components/common/Button';
import { Badge } from '../../components/common/Badge';
import { useScanStore } from '../../stores/scanStore';
import { useDriveStore } from '../../stores/driveStore';
import { formatBytes, formatNumber } from '../../utils/formatters';

const DEFAULT_COMMON_EXCLUSIONS = [
  'node_modules',
  '.git',
  '.Trash',
  '$Recycle.Bin',
  'System Volume Information',
  'AppData/Local/Temp',
];

export const ScanModal: React.FC = () => {
  const {
    isModalOpen,
    closeScanModal,
    targetDriveId,
    targetPath,
    setTargetPath,
    setTargetDriveId,
    exclusions,
    setExclusions,
    startScan,
    pauseScan,
    resumeScan,
    cancelScan,
    resetScan,
    isScanning,
    isPaused,
    progress,
    summary,
    error,
    fetchScanFiles,
  } = useScanStore();

  const { drives } = useDriveStore();

  const [customExclusion, setCustomExclusion] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Sync initial drive selection when modal opens
  useEffect(() => {
    if (isModalOpen && drives.length > 0) {
      const matchedDrive = drives.find((d) => d.id === targetDriveId);
      if (!matchedDrive) {
        setTargetDriveId(drives[0].id);
        if (!targetPath || targetPath === '/') {
          setTargetPath(drives[0].mountPoint);
        }
      } else if (!targetPath) {
        setTargetPath(matchedDrive.mountPoint);
      }
    }
  }, [isModalOpen, targetDriveId, drives, setTargetDriveId, targetPath, setTargetPath]);

  const handleDriveChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const selectedId = e.target.value;
    setTargetDriveId(selectedId);
    const drive = drives.find((d) => d.id === selectedId);
    if (drive) {
      setTargetPath(drive.mountPoint);
    }
  };

  const toggleExclusion = (item: string) => {
    if (exclusions.includes(item)) {
      setExclusions(exclusions.filter((e) => e !== item));
    } else {
      setExclusions([...exclusions, item]);
    }
  };

  const addCustomExclusion = () => {
    const trimmed = customExclusion.trim();
    if (trimmed && !exclusions.includes(trimmed)) {
      setExclusions([...exclusions, trimmed]);
      setCustomExclusion('');
    }
  };

  const handleStart = async () => {
    await startScan();
  };

  const handleFinishView = () => {
    if (summary) {
      fetchScanFiles(summary.sessionId, 1, 50, undefined, 'size', true);
    }
    closeScanModal();
  };

  const handleNewScan = () => {
    resetScan();
  };

  const formatElapsed = (millis: number) => {
    const seconds = millis / 1000;
    if (seconds < 60) {
      return `${seconds.toFixed(1)}s`;
    }
    const mins = Math.floor(seconds / 60);
    const remainingSecs = (seconds % 60).toFixed(0);
    return `${mins}m ${remainingSecs}s`;
  };

  const isCompleted = summary !== null;

  return (
    <Modal
      isOpen={isModalOpen}
      onClose={() => {
        if (!isScanning) {
          closeScanModal();
        }
      }}
      title={
        isScanning
          ? 'Filesystem Scan in Progress'
          : isCompleted
          ? 'Scan Completed'
          : 'Start Universal Storage Scan'
      }
      description={
        isScanning
          ? 'Controlled recursive directory crawler collecting file and folder metadata.'
          : isCompleted
          ? 'Scan finished and metadata indexed into SQLite.'
          : 'Configure target drive or path for recursive metadata indexing.'
      }
      maxWidth="lg"
      footer={
        <div className="flex items-center justify-between w-full">
          <div>
            {isScanning && (
              <span className="text-[11px] text-muted flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full bg-accent animate-ping" />
                Read-only analysis in background
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {!isScanning && !isCompleted && (
              <>
                <Button variant="ghost" onClick={closeScanModal}>
                  Cancel
                </Button>
                <Button
                  variant="primary"
                  icon={<Play className="w-3.5 h-3.5" />}
                  onClick={handleStart}
                  disabled={!targetPath.trim()}
                >
                  Start Scan
                </Button>
              </>
            )}

            {isScanning && (
              <>
                {isPaused ? (
                  <Button
                    variant="outline"
                    icon={<Play className="w-3.5 h-3.5" />}
                    onClick={resumeScan}
                  >
                    Resume
                  </Button>
                ) : (
                  <Button
                    variant="outline"
                    icon={<Pause className="w-3.5 h-3.5" />}
                    onClick={pauseScan}
                  >
                    Pause
                  </Button>
                )}
                <Button
                  variant="danger"
                  icon={<Square className="w-3.5 h-3.5" />}
                  onClick={cancelScan}
                >
                  Stop Scan
                </Button>
              </>
            )}

            {isCompleted && (
              <>
                <Button variant="ghost" onClick={handleNewScan} icon={<RefreshCw className="w-3.5 h-3.5" />}>
                  New Scan
                </Button>
                <Button
                  variant="primary"
                  onClick={handleFinishView}
                  icon={<CheckCircle2 className="w-3.5 h-3.5" />}
                >
                  View Indexed Files
                </Button>
              </>
            )}
          </div>
        </div>
      }
    >
      {/* 1. SETUP VIEW (Before Scan Starts) */}
      {!isScanning && !isCompleted && (
        <div className="space-y-4">
          {error && (
            <div className="p-3 bg-danger/10 border border-danger/20 rounded flex items-center gap-2 text-danger text-xs">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* Drive Selection */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-secondary flex items-center gap-1.5">
              <HardDrive className="w-3.5 h-3.5 text-accent" />
              Target Storage Volume
            </label>
            <select
              value={targetDriveId}
              onChange={handleDriveChange}
              className="w-full bg-elevated border border-border rounded px-3 py-2 text-xs text-primary focus:outline-none focus:border-accent"
            >
              {drives.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name || d.mountPoint} ({d.driveLetter || d.mountPoint}) - {d.fileSystem || 'Storage'}{' '}
                  {d.totalBytes ? `• ${formatBytes(d.totalBytes)}` : ''}
                </option>
              ))}
            </select>
          </div>

          {/* Root Path Input */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-secondary flex items-center gap-1.5">
              <FolderSearch className="w-3.5 h-3.5 text-accent" />
              Scan Root Path
            </label>
            <input
              type="text"
              value={targetPath}
              onChange={(e) => setTargetPath(e.target.value)}
              placeholder="e.g. C:\ or /Users/username or ~"
              className="w-full bg-elevated border border-border rounded px-3 py-2 font-mono text-xs text-primary focus:outline-none focus:border-accent"
            />
            {/* Quick Scope Presets */}
            <div className="flex flex-wrap items-center gap-1.5 pt-1">
              <span className="text-[10px] text-muted font-medium uppercase tracking-wider mr-1">Quick Target:</span>
              <button
                type="button"
                onClick={() => setTargetPath('~')}
                className={`px-2 py-0.5 rounded text-[11px] font-mono border transition-colors ${
                  targetPath === '~'
                    ? 'bg-accent/20 border-accent text-accent font-medium'
                    : 'bg-elevated border-border text-secondary hover:text-primary'
                }`}
              >
                ~ (Home - Recommended)
              </button>
              <button
                type="button"
                onClick={() => setTargetPath('/')}
                className={`px-2 py-0.5 rounded text-[11px] font-mono border transition-colors ${
                  targetPath === '/'
                    ? 'bg-accent/20 border-accent text-accent font-medium'
                    : 'bg-elevated border-border text-secondary hover:text-primary'
                }`}
              >
                / (Whole Drive)
              </button>
              <button
                type="button"
                onClick={() => setTargetPath('~/Downloads')}
                className="px-2 py-0.5 rounded text-[11px] font-mono border border-border bg-elevated text-secondary hover:text-primary"
              >
                ~/Downloads
              </button>
            </div>

            {targetPath === '/' && (
              <div className="p-2 bg-accent/5 border border-accent/20 rounded text-[11px] text-secondary flex items-start gap-1.5 mt-1">
                <span className="text-accent font-bold">Tip:</span>
                <span>
                  Scanning <strong>~</strong> (Home) is 10x faster and captures 95% of reclaimable space (caches, node_modules, build artifacts). Whole-drive (<strong>/</strong>) scanning requires macOS Full Disk Access to avoid system permissions skips.
                </span>
              </div>
            )}
            <p className="text-[11px] text-muted">
              Recursively visits subdirectories starting from this path. Symlink cycles and reparse junctions are automatically detected and safely skipped.
            </p>
          </div>

          {/* Exclusions Configuration */}
          <div className="space-y-2 pt-1 border-t border-border/40">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-secondary flex items-center gap-1.5">
                <Sliders className="w-3.5 h-3.5 text-secondary" />
                Directory Exclusions
              </label>
              <button
                type="button"
                onClick={() => setShowAdvanced(!showAdvanced)}
                className="text-[11px] text-accent hover:underline"
              >
                {showAdvanced ? 'Hide options' : 'Add custom rules'}
              </button>
            </div>

            <div className="flex flex-wrap gap-1.5">
              {DEFAULT_COMMON_EXCLUSIONS.map((item) => {
                const isSelected = exclusions.includes(item);
                return (
                  <button
                    key={item}
                    type="button"
                    onClick={() => toggleExclusion(item)}
                    className={`px-2 py-1 rounded text-[11px] border transition-colors ${
                      isSelected
                        ? 'bg-accent/15 border-accent/40 text-accent font-medium'
                        : 'bg-elevated/40 border-border text-secondary hover:text-primary'
                    }`}
                  >
                    {item}
                  </button>
                );
              })}
            </div>

            {showAdvanced && (
              <div className="flex gap-2 pt-1">
                <input
                  type="text"
                  value={customExclusion}
                  onChange={(e) => setCustomExclusion(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && addCustomExclusion()}
                  placeholder="e.g. .venv or target"
                  className="flex-1 bg-elevated border border-border rounded px-2.5 py-1.5 text-xs text-primary focus:outline-none focus:border-accent"
                />
                <Button size="sm" variant="outline" onClick={addCustomExclusion}>
                  Add Exclusion
                </Button>
              </div>
            )}
          </div>
        </div>
      )}

      {/* 2. ACTIVE PROGRESS VIEW (While Scanning) */}
      {isScanning && (
        <div className="space-y-4">
          {/* Status Header Pill */}
          <div className="flex items-center justify-between p-3 bg-elevated border border-border rounded">
            <div className="flex items-center gap-2">
              <span className={`w-2 h-2 rounded-full ${isPaused ? 'bg-warning' : 'bg-success animate-pulse'}`} />
              <span className="font-medium text-primary">
                {isPaused ? 'Scan Paused' : 'Traversing Filesystem...'}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <Clock className="w-3.5 h-3.5 text-muted" />
              <span className="font-mono text-xs text-secondary">
                {formatElapsed(progress?.elapsedMillis ?? 0)}
              </span>
              {isPaused && <Badge variant="warning">PAUSED</Badge>}
            </div>
          </div>

          {/* Metric Counter Cards */}
          <div className="grid grid-cols-4 gap-2">
            <div className="bg-elevated p-3 border border-border rounded">
              <div className="flex items-center gap-1.5 text-muted mb-1 text-[11px]">
                <Files className="w-3 h-3 text-accent" />
                Files Scanned
              </div>
              <div className="text-base font-semibold text-primary font-mono">
                {formatNumber(progress?.filesScanned ?? 0)}
              </div>
            </div>

            <div className="bg-elevated p-3 border border-border rounded">
              <div className="flex items-center gap-1.5 text-muted mb-1 text-[11px]">
                <FolderTree className="w-3 h-3 text-secondary" />
                Folders
              </div>
              <div className="text-base font-semibold text-primary font-mono">
                {formatNumber(progress?.dirsScanned ?? 0)}
              </div>
            </div>

            <div className="bg-elevated p-3 border border-border rounded">
              <div className="flex items-center gap-1.5 text-muted mb-1 text-[11px]">
                <Database className="w-3 h-3 text-success" />
                Logical Size
              </div>
              <div className="text-base font-semibold text-primary font-mono">
                {formatBytes(progress?.bytesScanned ?? 0)}
              </div>
            </div>

            <div className="bg-elevated p-3 border border-border rounded">
              <div className="flex items-center gap-1.5 text-muted mb-1 text-[11px]">
                <AlertTriangle className="w-3 h-3 text-warning" />
                Errors / Skips
              </div>
              <div className="text-base font-semibold font-mono text-warning">
                {formatNumber(progress?.errorCount ?? 0)}
              </div>
            </div>
          </div>

          {/* Current Path Monospace Line */}
          <div className="p-2.5 bg-elevated border border-border/80 rounded space-y-1">
            <div className="text-[10px] uppercase tracking-wider text-muted font-medium">
              Current Directory
            </div>
            <div
              className="text-xs font-mono text-secondary truncate"
              title={progress?.currentPath || targetPath}
            >
              {progress?.currentPath || targetPath}
            </div>
          </div>

          {/* Live Tracked File Feed */}
          <div className="p-2.5 bg-elevated border border-border/80 rounded space-y-2">
            <div className="flex items-center justify-between">
              <div className="text-[10px] uppercase tracking-wider text-muted font-medium flex items-center gap-1.5">
                <Activity className="w-3 h-3 text-accent animate-pulse" />
                Live Tracked Files
              </div>
              {progress?.currentFile && (
                <span className="text-[10px] text-accent font-mono truncate max-w-[280px]">
                  Tracking: {progress.currentFile.split('/').pop() || progress.currentFile}
                </span>
              )}
            </div>

            {progress?.currentFile ? (
              <div className="p-2 rounded bg-panel border border-accent/20 flex items-center gap-2">
                <FileText className="w-4 h-4 text-accent shrink-0 animate-pulse" />
                <div className="min-w-0 flex-1">
                  <div className="text-xs font-mono text-primary truncate font-medium">
                    {progress.currentFile.split('/').pop() || progress.currentFile}
                  </div>
                  <div className="text-[10px] font-mono text-muted truncate">
                    {progress.currentFile}
                  </div>
                </div>
                <Badge variant="accent" size="sm">
                  Active
                </Badge>
              </div>
            ) : (
              <div className="p-2 rounded bg-panel border border-border/50 flex items-center gap-2 text-xs font-mono text-secondary">
                <Activity className="w-3.5 h-3.5 text-accent animate-pulse shrink-0" />
                <span className="truncate">{progress?.currentPath || targetPath || 'Traversing filesystem...'}</span>
              </div>
            )}

            {/* Recent Tracked Files Stream */}
            {progress?.recentFiles && progress.recentFiles.length > 0 && (
              <div className="space-y-1 pt-1 border-t border-border/40">
                <div className="text-[10px] text-muted font-medium uppercase tracking-wider">
                  Recently Tracked ({progress.recentFiles.length})
                </div>
                <div className="space-y-1 max-h-28 overflow-y-auto pr-1">
                  {progress.recentFiles.slice().reverse().map((filePath, idx) => {
                    const fileName = filePath.split('/').pop() || filePath;
                    return (
                      <div
                        key={`${filePath}-${idx}`}
                        className="flex items-center justify-between text-[11px] font-mono text-secondary py-1 px-2 rounded bg-panel/60 border border-border/30 hover:border-accent/30 transition-colors"
                      >
                        <span className="truncate flex-1 text-primary" title={filePath}>
                          {fileName}
                        </span>
                        <span className="text-[10px] text-muted truncate max-w-[180px] ml-2 text-right">
                          {filePath.substring(0, filePath.lastIndexOf('/')) || '/'}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          {/* Error notice if errors exist */}
          {(progress?.errorCount ?? 0) > 0 && (
            <div className="p-2.5 bg-warning/10 border border-warning/20 rounded text-[11px] text-warning flex items-center gap-2">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span>
                {progress?.errorCount} protected or inaccessible items skipped safely (e.g. permission-denied OS paths).
              </span>
            </div>
          )}
        </div>
      )}

      {/* 3. COMPLETION SUMMARY VIEW */}
      {isCompleted && summary && (
        <div className="space-y-4">
          <div
            className={`p-3 rounded border flex items-center gap-3 ${
              summary.status === 'completedWithErrors'
                ? 'bg-warning/10 border-warning/30 text-warning'
                : 'bg-success/10 border-success/30 text-success'
            }`}
          >
            {summary.status === 'completedWithErrors' ? (
              <AlertTriangle className="w-5 h-5 shrink-0" />
            ) : (
              <CheckCircle2 className="w-5 h-5 shrink-0" />
            )}
            <div>
              <div className="font-semibold text-xs text-primary">
                {summary.status === 'completedWithErrors'
                  ? 'Scan Finished with Permission Warnings'
                  : 'Filesystem Scan Finished Successfully'}
              </div>
              <div className="text-[11px] text-secondary mt-0.5">
                Indexed in {formatElapsed(summary.durationMillis)}. Batch commits completed and metadata persisted in SQLite.
              </div>
            </div>
          </div>

          {/* Summary Breakdown Grid */}
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="p-3 bg-elevated border border-border rounded flex justify-between items-center">
              <span className="text-secondary">Total Files Discovered:</span>
              <span className="font-semibold text-primary font-mono">{formatNumber(summary.totalFiles)}</span>
            </div>
            <div className="p-3 bg-elevated border border-border rounded flex justify-between items-center">
              <span className="text-secondary">Directories Traversed:</span>
              <span className="font-semibold text-primary font-mono">{formatNumber(summary.totalDirs)}</span>
            </div>
            <div className="p-3 bg-elevated border border-border rounded flex justify-between items-center">
              <span className="text-secondary">Total Consumed Storage:</span>
              <span className="font-semibold text-accent font-mono">{formatBytes(summary.totalBytes)}</span>
            </div>
            <div className="p-3 bg-elevated border border-border rounded flex justify-between items-center">
              <span className="text-secondary">Scan Duration:</span>
              <span className="font-semibold text-primary font-mono">{formatElapsed(summary.durationMillis)}</span>
            </div>
          </div>

          {summary.errorCount > 0 && (
            <div className="p-3 bg-elevated border border-border rounded space-y-1">
              <div className="flex items-center justify-between text-xs">
                <span className="text-secondary">Permission & Traversal Skips:</span>
                <span className="text-warning font-mono font-medium">{formatNumber(summary.errorCount)} paths</span>
              </div>
              <p className="text-[11px] text-muted">
                System protected directories or unreadable handles were logged in the database without halting traversal.
              </p>
            </div>
          )}

          <div className="p-2.5 bg-elevated border border-border/80 rounded space-y-1 text-xs">
            <span className="text-muted block text-[10px] uppercase tracking-wider">Indexed Root</span>
            <span className="font-mono text-secondary break-all">{summary.rootPath}</span>
          </div>
        </div>
      )}
    </Modal>
  );
};

