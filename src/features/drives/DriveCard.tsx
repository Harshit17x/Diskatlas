import React from 'react';
import {
  HardDrive,
  Server,
  Usb,
  CreditCard,
  Network,
  Disc,
  Info,
  Play,
} from 'lucide-react';
import type { DriveInfo, DriveType } from '../../types/bindings';
import { formatBytes, formatPercent } from '../../utils/formatters';
import { Card } from '../../components/common/Card';
import { Badge } from '../../components/common/Badge';
import { useDriveStore } from '../../stores/driveStore';
import { useScanStore } from '../../stores/scanStore';

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
  const { openDetailModal } = useDriveStore();

  const getProgressColor = (percent: number) => {
    if (percent >= 90) return 'bg-danger';
    if (percent >= 75) return 'bg-warning';
    return 'bg-accent';
  };

  const getDriveIcon = (type: DriveType) => {
    switch (type) {
      case 'internalSsd':
        return <Server className="w-4 h-4 text-accent" />;
      case 'internalHdd':
        return <HardDrive className="w-4 h-4 text-secondary" />;
      case 'externalSsd':
      case 'externalHdd':
        return <HardDrive className="w-4 h-4 text-warning" />;
      case 'usb':
        return <Usb className="w-4 h-4 text-accent" />;
      case 'sdCard':
        return <CreditCard className="w-4 h-4 text-warning" />;
      case 'network':
        return <Network className="w-4 h-4 text-accent" />;
      case 'optical':
        return <Disc className="w-4 h-4 text-secondary" />;
      default:
        return <HardDrive className="w-4 h-4 text-secondary" />;
    }
  };

  const getDriveTypeBadge = (type: DriveType) => {
    switch (type) {
      case 'internalSsd':
        return <Badge variant="accent" size="sm">Internal SSD</Badge>;
      case 'internalHdd':
        return <Badge variant="neutral" size="sm">Internal HDD</Badge>;
      case 'externalSsd':
        return <Badge variant="warning" size="sm">External SSD</Badge>;
      case 'externalHdd':
        return <Badge variant="warning" size="sm">External HDD</Badge>;
      case 'usb':
        return <Badge variant="accent" size="sm">USB Flash</Badge>;
      case 'sdCard':
        return <Badge variant="warning" size="sm">SD Card</Badge>;
      case 'network':
        return <Badge variant="neutral" size="sm">Network</Badge>;
      case 'optical':
        return <Badge variant="neutral" size="sm">Optical</Badge>;
      default:
        return <Badge variant="neutral" size="sm">Volume</Badge>;
    }
  };

  const handleInspect = (e: React.MouseEvent) => {
    e.stopPropagation();
    openDetailModal(drive.id);
  };

  return (
    <Card
      hoverEffect
      selected={isSelected}
      onClick={() => onSelect(drive.id)}
      padding="md"
      className="flex flex-col justify-between"
    >
      <div>
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div
              className={`w-9 h-9 rounded flex items-center justify-center ${
                isSelected ? 'bg-accent text-white' : 'bg-elevated text-secondary'
              }`}
            >
              {getDriveIcon(drive.driveType)}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-xs font-semibold text-primary">{drive.name}</h3>
                {drive.driveLetter ? (
                  <span className="text-xs font-mono text-accent font-bold">
                    [{drive.driveLetter}]
                  </span>
                ) : (
                  <span className="text-xs font-mono text-muted">({drive.mountPoint})</span>
                )}
              </div>
              <p className="text-[11px] text-secondary font-mono mt-0.5">
                {drive.fileSystem || 'Unknown FS'} • ID: {drive.id}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-1.5">
            {drive.isSystem && <Badge variant="accent" size="sm">System OS</Badge>}
            {getDriveTypeBadge(drive.driveType)}
            {drive.isReadOnly && <Badge variant="warning" size="sm">RO</Badge>}
          </div>
        </div>

        {/* Storage Meter */}
        <div className="mt-4 space-y-1.5">
          {drive.totalBytes !== null ? (
            <>
              <div className="flex justify-between text-xs">
                <span className="text-secondary">
                  Used:{' '}
                  <strong className="text-primary font-mono">
                    {drive.usedBytes !== null ? formatBytes(drive.usedBytes) : '—'}
                  </strong>
                </span>
                <span className="text-secondary">
                  Free:{' '}
                  <strong className="text-primary font-mono">
                    {drive.availableBytes !== null ? formatBytes(drive.availableBytes) : '—'}
                  </strong>
                </span>
              </div>

              <div className="h-1.5 w-full bg-elevated rounded-sm overflow-hidden border border-border">
                <div
                  className={`h-full rounded-sm transition-all duration-300 ${
                    drive.usedPercent !== null ? getProgressColor(drive.usedPercent) : 'bg-accent'
                  }`}
                  style={{ width: `${Math.min(drive.usedPercent ?? 0, 100)}%` }}
                />
              </div>

              <div className="flex justify-between items-center text-[10px] text-muted font-mono">
                <span>
                  {drive.usedPercent !== null ? `${formatPercent(drive.usedPercent)} consumed` : '—'}
                </span>
                <span>Capacity: {formatBytes(drive.totalBytes)}</span>
              </div>
            </>
          ) : (
            <div className="py-2.5 px-3 bg-elevated border border-border rounded text-[11px] text-muted font-mono text-center">
              Capacity unavailable (Unmounted or empty media)
            </div>
          )}
        </div>
      </div>

      {/* Action buttons footer */}
      <div className="mt-3 pt-2 border-t border-border flex items-center justify-between text-[11px]">
        <span className="text-muted font-mono truncate max-w-[130px]" title={drive.mountPoint}>
          {drive.mountPoint}
        </span>
        <div className="flex items-center gap-3">
          <button
            onClick={(e) => {
              e.stopPropagation();
              useScanStore.getState().openScanModal(drive.mountPoint, drive.id);
            }}
            className="text-primary hover:text-accent font-medium flex items-center gap-1 transition-colors"
            title="Scan this drive"
          >
            <Play className="w-3 h-3 text-accent" />
            <span>Scan</span>
          </button>
          <button
            onClick={handleInspect}
            className="text-secondary hover:text-primary font-medium flex items-center gap-1 transition-colors"
            title="Inspect low-level volume diagnostics"
          >
            <Info className="w-3 h-3" />
            <span>Diagnostics</span>
          </button>
        </div>
      </div>
    </Card>
  );
};
