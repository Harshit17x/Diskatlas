import React from 'react';
import { HardDrive, Server, Usb } from 'lucide-react';
import type { DriveInfo } from '../../types/bindings';
import { formatBytes, formatPercent } from '../../utils/formatters';
import { Card } from '../../components/common/Card';
import { Badge } from '../../components/common/Badge';

export interface DriveCardProps {
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
    if (percent >= 90) return 'bg-danger';
    if (percent >= 75) return 'bg-warning';
    return 'bg-accent';
  };

  return (
    <Card
      hoverEffect
      selected={isSelected}
      onClick={() => onSelect(drive.id)}
      padding="md"
    >
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-3">
          <div
            className={`w-9 h-9 rounded flex items-center justify-center ${
              isSelected ? 'bg-accent text-white' : 'bg-elevated text-secondary'
            }`}
          >
            {drive.isRemovable ? (
              <Usb className="w-4 h-4" />
            ) : drive.isSystem ? (
              <Server className="w-4 h-4" />
            ) : (
              <HardDrive className="w-4 h-4" />
            )}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h3 className="text-xs font-semibold text-primary">{drive.label}</h3>
              <span className="text-xs font-mono text-muted">({drive.mountPoint})</span>
            </div>
            <p className="text-[11px] text-secondary font-mono mt-0.5">
              {drive.fileSystem} • ID: {drive.id}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1.5">
          {drive.isSystem && <Badge variant="accent" size="sm">System OS</Badge>}
          {drive.isRemovable && <Badge variant="warning" size="sm">Removable</Badge>}
        </div>
      </div>

      {/* Storage Meter */}
      <div className="mt-4 space-y-1.5">
        <div className="flex justify-between text-xs">
          <span className="text-secondary">
            Used: <strong className="text-primary font-mono">{formatBytes(drive.usedBytes)}</strong>
          </span>
          <span className="text-secondary">
            Free: <strong className="text-primary font-mono">{formatBytes(drive.availableBytes)}</strong>
          </span>
        </div>

        <div className="h-1.5 w-full bg-[#121417] rounded-sm overflow-hidden border border-border">
          <div
            className={`h-full rounded-sm transition-all duration-300 ${getProgressColor(
              drive.usedPercent
            )}`}
            style={{ width: `${Math.min(drive.usedPercent, 100)}%` }}
          />
        </div>

        <div className="flex justify-between items-center text-[10px] text-muted font-mono">
          <span>{formatPercent(drive.usedPercent)} consumed</span>
          <span>Capacity: {formatBytes(drive.totalBytes)}</span>
        </div>
      </div>
    </Card>
  );
};
