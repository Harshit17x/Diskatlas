import React, { useState } from 'react';
import { Modal } from '../common/Modal';
import { Button } from '../common/Button';
import { Badge } from '../common/Badge';
import { useCleanupStore } from '../../stores/cleanupStore';
import { formatBytes } from '../../utils/formatters';
import {
  AlertTriangle,
  Trash2,
  CloudOff,
  RotateCcw,
  CheckCircle2,
  Loader2,
  FileText,
  AlertOctagon,
} from 'lucide-react';

export const CleanupConfirmationModal: React.FC = () => {
  const {
    cleanupPlan,
    selectedIds,
    useRecycleBin,
    setUseRecycleBin,
    confirmationModalOpen,
    setConfirmationModalOpen,
    executeCleanup,
    executingCleanup,
    error,
  } = useCleanupStore();

  const [inputPhrase, setInputPhrase] = useState('');

  if (!cleanupPlan || !confirmationModalOpen) return null;

  const selectedCandidates = cleanupPlan.candidates.filter((c) => selectedIds.has(c.id));
  const totalReclaimableBytes = selectedCandidates.reduce((acc, c) => acc + c.sizeBytes, 0);
  const cloudSyncedItems = selectedCandidates.filter((c) => c.isCloudSynced);
  const reviewRequiredItems = selectedCandidates.filter((c) => c.group === 'review_required');

  const isConfirmed = inputPhrase.trim() === 'CONFIRM_CLEANUP';

  const handleClose = () => {
    if (executingCleanup) return;
    setInputPhrase('');
    setConfirmationModalOpen(false);
  };

  const handleExecute = async () => {
    if (!isConfirmed || executingCleanup) return;
    const success = await executeCleanup(inputPhrase.trim());
    if (success) {
      setInputPhrase('');
    }
  };

  return (
    <Modal
      isOpen={confirmationModalOpen}
      onClose={handleClose}
      title="Confirm Storage Cleanup"
      description="Review selected items and verify safety parameters before performing cleanup"
      maxWidth="lg"
      footer={
        <div className="flex items-center justify-between w-full">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleClose}
            disabled={executingCleanup}
          >
            Cancel
          </Button>
          <Button
            variant="danger"
            size="sm"
            onClick={handleExecute}
            disabled={!isConfirmed || executingCleanup}
            icon={
              executingCleanup ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Trash2 className="w-4 h-4" />
              )
            }
          >
            {executingCleanup ? 'Cleaning up...' : 'Execute Confirmed Cleanup'}
          </Button>
        </div>
      }
    >
      <div className="space-y-4 max-h-[68vh] overflow-y-auto pr-1">
        {/* Error notification */}
        {error && (
          <div className="p-3 bg-[#DB6262]/10 border border-[#DB6262]/30 rounded text-xs text-[#DB6262] flex items-start gap-2">
            <AlertOctagon className="w-4 h-4 shrink-0 mt-0.5" />
            <span>{error}</span>
          </div>
        )}

        {/* 1. Summary Metrics */}
        <div className="grid grid-cols-2 gap-3 p-3 bg-elevated/40 border border-border rounded">
          <div>
            <span className="text-[11px] text-secondary uppercase font-mono">Selected Items</span>
            <div className="text-base font-semibold text-primary mt-0.5">
              {selectedCandidates.length} items
            </div>
          </div>
          <div>
            <span className="text-[11px] text-secondary uppercase font-mono">Estimated Reclaimed</span>
            <div className="text-base font-semibold text-accent mt-0.5">
              {formatBytes(totalReclaimableBytes)}
            </div>
          </div>
        </div>

        {/* 2. Reversible Mode Toggle */}
        <div className="p-3 bg-panel border border-border rounded space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <RotateCcw className="w-4 h-4 text-success" />
              <span className="text-xs font-semibold text-primary">
                Reversible Mode (Recycle Bin / Trash)
              </span>
            </div>
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={useRecycleBin}
                onChange={(e) => setUseRecycleBin(e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-border peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-success"></div>
            </label>
          </div>
          <p className="text-[11px] text-secondary leading-relaxed">
            {useRecycleBin ? (
              <span className="text-success">
                Items will be sent to the system Recycle Bin / Trash, allowing you to easily restore them if needed.
              </span>
            ) : (
              <span className="text-[#DB6262] font-semibold">
                Permanent deletion: Items will be removed directly from the disk without moving to the Recycle Bin.
              </span>
            )}
          </p>
        </div>

        {/* 3. Cloud Synchronization Warning */}
        {cloudSyncedItems.length > 0 && (
          <div className="p-3 bg-warning/10 border border-warning/30 rounded flex items-start gap-2.5">
            <CloudOff className="w-4 h-4 text-warning shrink-0 mt-0.5" />
            <div className="text-xs space-y-1">
              <span className="font-semibold text-warning">
                Cloud Sync Propagation Notice ({cloudSyncedItems.length} items)
              </span>
              <p className="text-[11px] text-secondary">
                One or more selected files reside in cloud-synced folders (OneDrive, Google Drive, Dropbox, iCloud). Deleting them will sync deletions across your cloud storage and connected devices.
              </p>
            </div>
          </div>
        )}

        {/* 4. Review Required Notice */}
        {reviewRequiredItems.length > 0 && (
          <div className="p-3 bg-elevated border border-border rounded flex items-start gap-2.5">
            <AlertTriangle className="w-4 h-4 text-warning shrink-0 mt-0.5" />
            <div className="text-xs space-y-1">
              <span className="font-semibold text-primary">
                {reviewRequiredItems.length} items require user discretion
              </span>
              <p className="text-[11px] text-secondary">
                These files may include project outputs, archives, or application caches that might need to be recompiled or redownloaded if required later.
              </p>
            </div>
          </div>
        )}

        {/* 5. Exact Files Selected Preview */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between text-xs text-secondary font-mono">
            <span>PREVIEW OF ITEMS TO CLEAN ({selectedCandidates.length})</span>
            <span>TOTAL: {formatBytes(totalReclaimableBytes)}</span>
          </div>
          <div className="max-h-44 overflow-y-auto border border-border rounded divide-y divide-border bg-[#101114]">
            {selectedCandidates.map((c) => (
              <div key={c.id} className="p-2 flex items-center justify-between gap-3 text-xs">
                <div className="min-w-0 flex items-center gap-2">
                  <FileText className="w-3.5 h-3.5 text-muted shrink-0" />
                  <div className="truncate">
                    <span className="font-medium text-primary block truncate">{c.name}</span>
                    <span className="text-[10px] text-secondary font-mono block truncate" title={c.path}>
                      {c.path}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <Badge variant="neutral" size="sm">
                    {c.category}
                  </Badge>
                  <span className="font-mono text-xs font-semibold text-accent">
                    {formatBytes(c.sizeBytes)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* 6. Explicit Confirmation Phrase */}
        <div className="p-3 bg-elevated/70 border border-border rounded space-y-2">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-accent" />
            <span className="text-xs font-semibold text-primary">
              Explicit Safety Confirmation
            </span>
          </div>
          <p className="text-[11px] text-secondary">
            To prevent accidental or automated deletions, please type{' '}
            <code className="text-accent bg-panel px-1.5 py-0.5 rounded font-mono font-bold">
              CONFIRM_CLEANUP
            </code>{' '}
            below:
          </p>
          <input
            type="text"
            value={inputPhrase}
            onChange={(e) => setInputPhrase(e.target.value)}
            placeholder="Type CONFIRM_CLEANUP here"
            className="w-full bg-[#101114] border border-border rounded px-3 py-1.5 text-xs text-primary font-mono focus:outline-none focus:border-accent"
          />
        </div>
      </div>
    </Modal>
  );
};
