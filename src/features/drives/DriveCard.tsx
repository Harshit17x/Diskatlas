import React from 'react';
import { HardDrive, Server, Usb } from 'lucide-react';
import type { DriveInfo } from '../../types/bindings';
import { formatBytes, formatPercent } from '../../utils/formatters';
import { Card } from '../../components/common/Card';
import { Badge } from '../../components/common/Badge';

interface DriveCardProps {
  drive: DriveInfo;
  isSelected: boolean;
  onSelect: (id: string) => void;
}

export const DriveCard: React.FC<DriveCardProps> = ({
  drive,
  isSelected,
  onSelect,
}) => {
  const getProgressColor = (percent: number) => {
    if (percent >= 90) return 'bg-rose-500';
    if (percent >= 75) return 'bg-amber-500';
    return 'bg-sky-500';
  };

  return (
    <Card
      hoverEffect
      onClick={() => onSelect(drive.id)}
      className={`border transition-all ${
        isSelected
          ? 'border-sky-500 bg-slate-900/90 shadow-sky-500/10'
          : 'border-slate-800/80 hover:border-slate-700'
      }`}
    >
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div
            className={`w-10 h-10 rounded-lg flex items-center justify-center ${
              isSelected ? 'bg-sky-500/20 text-sky-400' : 'bg-slate-800 text-slate-400'
            }`}
          >
            {drive.isRemovable ? (
              <Usb className="w-5 h-5" />
            ) : drive.isSystem ? (
              <Server className="w-5 h-5" />
            ) : (
              <HardDrive className="w-5 h-5" />
            )}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h3 className="text-sm font-semibold text-slate-100">{drive.label}</h3>
              <span className="text-xs font-mono text-slate-400">({drive.mountPoint})</span>
            </div>
            <p className="text-xs text-slate-500 font-mono mt-0.5">
              FS: {drive.fileSystem} • ID: {drive.id}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1.5">
          {drive.isSystem && <Badge variant="primary">System OS</Badge>}
          {drive.isRemovable && <Badge variant="warning">Removable</Badge>}
        </div>
      </div>

      {/* Storage Meter */}
      <div className="mt-5 space-y-2">
        <div className="flex justify-between text-xs">
          <span className="text-slate-400">
            Used: <strong className="text-slate-200">{formatBytes(drive.usedBytes)}</strong>
          </span>
          <span className="text-slate-400">
            Available: <strong className="text-slate-200">{formatBytes(drive.availableBytes)}</strong>
          </span>
        </div>

        <div className="h-2 w-full bg-slate-800 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-500 ${getProgressColor(
              drive.usedPercent
            )}`}
            style={{ width: `${Math.min(drive.usedPercent, 100)}%` }}
          />
        </div>

        <div className="flex justify-between items-center text-[11px] text-slate-500 font-mono">
          <span>{formatPercent(drive.usedPercent)} capacity consumed</span>
          <span>Total {formatBytes(drive.totalBytes)}</span>
        </div>
      </div>
    </Card>
  );
};

