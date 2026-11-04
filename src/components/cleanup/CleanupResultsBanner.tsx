import React from 'react';
import { useCleanupStore } from '../../stores/cleanupStore';
import { formatBytes } from '../../utils/formatters';
import { CheckCircle2, AlertTriangle, X, ShieldAlert } from 'lucide-react';

export const CleanupResultsBanner: React.FC = () => {
  const { executionResponse, resetExecutionResponse } = useCleanupStore();

  if (!executionResponse) return null;

  const { successfulCount, failedCount, bytesReclaimed, results } = executionResponse;
  const blockedItems = results.filter((r) => r.status === 'blocked_security');

  return (
    <div className="p-4 bg-panel border border-border rounded shadow-lg space-y-3 animate-in fade-in duration-200">
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2.5">
          {failedCount === 0 ? (
            <div className="p-1.5 bg-success/20 text-success rounded-full">
              <CheckCircle2 className="w-5 h-5" />
            </div>
          ) : (
            <div className="p-1.5 bg-warning/20 text-warning rounded-full">
              <AlertTriangle className="w-5 h-5" />
            </div>
          )}
          <div>
            <h3 className="text-sm font-semibold text-primary">
              Cleanup Completed: Reclaimed {formatBytes(bytesReclaimed)}
            </h3>
            <p className="text-xs text-secondary mt-0.5">
              {successfulCount} items successfully removed
              {failedCount > 0 && `, ${failedCount} items failed or were blocked`}
            </p>
          </div>
        </div>

        <button
          onClick={resetExecutionResponse}
          className="text-muted hover:text-primary transition-colors p-1"
          title="Dismiss"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {blockedItems.length > 0 && (
        <div className="p-2.5 bg-warning/10 border border-warning/20 rounded text-xs space-y-1">
          <div className="flex items-center gap-1.5 font-semibold text-warning">
            <ShieldAlert className="w-4 h-4" />
            <span>Safeguards Blocked {blockedItems.length} Operating System Items</span>
          </div>
          <p className="text-secondary text-[11px]">
            DiskAtlas security safeguards prevented the modification of protected system components.
          </p>
        </div>
      )}

      {/* Item results scroll list */}
      <div className="max-h-32 overflow-y-auto border border-border rounded divide-y divide-border bg-[#101114] text-xs">
        {results.map((r, i) => (
          <div key={i} className="p-2 flex items-center justify-between gap-2">
            <div className="truncate">
              <span className="font-mono text-primary text-[11px] block truncate">{r.name}</span>
              <span className="font-mono text-secondary text-[10px] block truncate" title={r.path}>
                {r.path}
              </span>
            </div>
            <div className="shrink-0 flex items-center gap-2">
              <span className="font-mono text-[11px] text-accent">{formatBytes(r.sizeBytes)}</span>
              <span
                className={`text-[10px] uppercase font-mono px-1.5 py-0.5 rounded ${
                  r.status === 'success'
                    ? 'bg-success/20 text-success'
                    : r.status === 'blocked_security'
                    ? 'bg-warning/20 text-warning'
                    : 'bg-[#DB6262]/20 text-[#DB6262]'
                }`}
              >
                {r.status.replace('_', ' ')}
              </span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
