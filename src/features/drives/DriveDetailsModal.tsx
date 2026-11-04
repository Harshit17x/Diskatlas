import React from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { Modal } from '../../components/common/Modal';
import { Badge } from '../../components/common/Badge';
import { Button } from '../../components/common/Button';
import { formatBytes, formatPercent } from '../../utils/formatters';
import {
  HardDrive,
  Server,
  Usb,
  CreditCard,
  Network,
  Disc,
  ShieldCheck,
} from 'lucide-react';

export const DriveDetailsModal: React.FC = () => {
  const { isDetailModalOpen, activeDriveDetails, isInspecting, closeDetailModal } = useDriveStore();

  if (!isDetailModalOpen) return null;

  const drive = activeDriveDetails?.drive;

  const getDriveIcon = (type?: string) => {
    switch (type) {
      case 'internalSsd':
        return <Server className="w-5 h-5 text-accent" />;
      case 'internalHdd':
        return <HardDrive className="w-5 h-5 text-secondary" />;
      case 'externalSsd':
      case 'externalHdd':
        return <HardDrive className="w-5 h-5 text-warning" />;
      case 'usb':
        return <Usb className="w-5 h-5 text-accent" />;
      case 'sdCard':
        return <CreditCard className="w-5 h-5 text-warning" />;
      case 'network':
        return <Network className="w-5 h-5 text-accent" />;
      case 'optical':
        return <Disc className="w-5 h-5 text-secondary" />;
      default:
        return <HardDrive className="w-5 h-5 text-secondary" />;
    }
  };

  const formatDriveType = (type?: string) => {
    switch (type) {
      case 'internalSsd':
        return 'Internal SSD (Solid-State)';
      case 'internalHdd':
        return 'Internal HDD (Hard Disk)';
      case 'externalSsd':
        return 'External SSD';
      case 'externalHdd':
        return 'External HDD';
      case 'usb':
        return 'USB Removable Flash';
      case 'sdCard':
        return 'Memory Card (SD/MMC)';
      case 'network':
        return 'Network Volume';
      case 'optical':
        return 'Optical CD/DVD Media';
      case 'ram':
        return 'RAM Disk';
      default:
        return 'Mounted Storage Volume';
    }
  };

  const clusterSize =
    activeDriveDetails?.sectorsPerCluster && activeDriveDetails?.bytesPerSector
      ? activeDriveDetails.sectorsPerCluster * activeDriveDetails.bytesPerSector
      : null;

  return (
    <Modal
      isOpen={isDetailModalOpen}
      onClose={closeDetailModal}
      title={drive ? `${drive.name} — Hardware & Volume Diagnostics` : 'Inspecting Volume...'}
      description="Low-level operating system storage parameters and volume descriptors"
      maxWidth="lg"
      footer={
        <Button variant="secondary" size="sm" onClick={closeDetailModal}>
          Close Diagnostics
        </Button>
      }
    >
      {isInspecting ? (
        <div className="flex flex-col items-center justify-center py-10 text-secondary">
          <div className="w-6 h-6 border-2 border-accent border-t-transparent rounded-full animate-spin mb-3"></div>
          <p className="text-xs">Querying operating system volume descriptors...</p>
        </div>
      ) : drive ? (
        <div className="space-y-4">
          {/* Top Summary Banner */}
          <div className="p-3 bg-[#15171B] border border-border rounded flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded bg-elevated border border-border flex items-center justify-center">
                {getDriveIcon(drive.driveType)}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-semibold text-primary">{drive.name}</h3>
                  {drive.driveLetter && (
                    <span className="text-xs font-mono text-accent font-bold">
                      [{drive.driveLetter}]
                    </span>
                  )}
                </div>
                <p className="text-xs text-secondary font-mono mt-0.5">{drive.mountPoint}</p>
              </div>
            </div>

            <div className="flex flex-col items-end gap-1">
              <Badge variant={drive.isSystem ? 'accent' : 'neutral'} size="sm">
                {formatDriveType(drive.driveType)}
              </Badge>
              {drive.isReadOnly && <Badge variant="warning" size="sm">Read-Only</Badge>}
            </div>
          </div>

          {/* Capacity Diagnostic Section */}
          <div className="p-3 bg-[#14161A] border border-border rounded space-y-2">
            <span className="text-[10px] uppercase font-semibold text-muted tracking-wider block">
              Storage Allocation Metrics
            </span>

            {drive.totalBytes !== null ? (
              <div className="space-y-1.5 font-mono text-xs">
                <div className="flex justify-between text-secondary">
                  <span>Capacity:</span>
                  <span className="text-primary font-bold">
                    {formatBytes(drive.totalBytes)} ({drive.totalBytes.toLocaleString()} bytes)
                  </span>
                </div>
                <div className="flex justify-between text-secondary">
                  <span>Used Space:</span>
                  <span className="text-primary">
                    {drive.usedBytes !== null ? formatBytes(drive.usedBytes) : '—'}
                    {drive.usedPercent !== null ? ` (${formatPercent(drive.usedPercent)})` : ''}
                  </span>
                </div>
                <div className="flex justify-between text-secondary">
                  <span>Free Available:</span>
                  <span className="text-success font-semibold">
                    {drive.availableBytes !== null ? formatBytes(drive.availableBytes) : '—'}
                  </span>
                </div>

                {drive.usedPercent !== null && (
                  <div className="h-1.5 w-full bg-[#1B1E23] rounded-sm overflow-hidden border border-border mt-2">
                    <div
                      className={`h-full rounded-sm ${
                        drive.usedPercent >= 90
                          ? 'bg-danger'
                          : drive.usedPercent >= 75
                          ? 'bg-warning'
                          : 'bg-accent'
                      }`}
                      style={{ width: `${Math.min(drive.usedPercent, 100)}%` }}
                    />
                  </div>
                )}
              </div>
            ) : (
              <div className="py-2 text-warning text-xs">
                Capacity values are currently unavailable (unmounted media card or empty tray).
              </div>
            )}
          </div>

          {/* Hardware Parameters Grid */}
          <div className="grid grid-cols-2 gap-2 text-xs font-mono">
            <div className="p-2.5 bg-[#15171B] border border-border rounded">
              <span className="text-[10px] text-muted uppercase font-sans block">Filesystem</span>
              <span className="text-primary font-semibold mt-0.5 block">
                {drive.fileSystem || 'Raw / Unknown'}
              </span>
            </div>

            <div className="p-2.5 bg-[#15171B] border border-border rounded">
              <span className="text-[10px] text-muted uppercase font-sans block">Bus Interface</span>
              <span className="text-primary font-semibold mt-0.5 block truncate">
                {activeDriveDetails?.busType || 'OS Storage Bus'}
              </span>
            </div>

            <div className="p-2.5 bg-[#15171B] border border-border rounded">
              <span className="text-[10px] text-muted uppercase font-sans block">Allocation Unit (Cluster)</span>
              <span className="text-primary mt-0.5 block">
                {clusterSize ? `${clusterSize} bytes (${formatBytes(clusterSize)})` : 'Default'}
              </span>
            </div>

            <div className="p-2.5 bg-[#15171B] border border-border rounded">
              <span className="text-[10px] text-muted uppercase font-sans block">Volume Identifier / Serial</span>
              <span className="text-primary mt-0.5 block truncate">
                {drive.volumeId || drive.id}
              </span>
            </div>
          </div>

          {/* Flags and Security */}
          {activeDriveDetails?.flags && activeDriveDetails.flags.length > 0 && (
            <div className="space-y-1">
              <span className="text-[10px] uppercase font-semibold text-muted tracking-wider block">
                Volume Descriptor Flags
              </span>
              <div className="flex flex-wrap gap-1.5">
                {activeDriveDetails.flags.map((flag) => (
                  <Badge key={flag} variant="neutral" size="sm">
                    {flag}
                  </Badge>
                ))}
                {drive.isSystem && (
                  <Badge variant="accent" size="sm">
                    <ShieldCheck className="w-3 h-3 text-accent mr-1" />
                    OS System Volume
                  </Badge>
                )}
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="py-6 text-center text-xs text-danger">
          Could not retrieve detailed hardware metrics for this drive.
        </div>
      )}
    </Modal>
  );
};
