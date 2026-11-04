import React from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { DriveCard } from './DriveCard';
import { HardDrive, AlertCircle } from 'lucide-react';
import { Button } from '../../components/common/Button';

export const DriveList: React.FC = () => {
  const { drives, selectedDriveId, selectDrive, isLoading, error, fetchDrives } = useDriveStore();

  if (isLoading && drives.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-secondary">
        <div className="w-6 h-6 border-2 border-accent border-t-transparent rounded-full animate-spin mb-3"></div>
        <p className="text-xs">Enumerating connected storage volumes...</p>
      </div>
    );
  }

  if (error && drives.length === 0) {
    return (
      <div className="p-4 rounded bg-panel border border-danger text-danger flex items-start gap-3">
        <AlertCircle className="w-4 h-4 flex-shrink-0 mt-0.5" />
        <div className="flex-1">
          <h4 className="font-semibold text-xs">Drive Detection Failed</h4>
          <p className="text-[11px] text-secondary mt-1">{error}</p>
        </div>
        <Button variant="outline" size="sm" onClick={fetchDrives}>
          Retry
        </Button>
      </div>
    );
  }

  if (drives.length === 0) {
    return (
      <div className="text-center py-12 border border-dashed border-border rounded bg-panel/30">
        <HardDrive className="w-8 h-8 text-muted mx-auto mb-2" />
        <h3 className="text-xs font-semibold text-primary">No volumes detected</h3>
        <p className="text-[11px] text-secondary mt-1 max-w-sm mx-auto">
          Verify device permissions or click refresh to re-enumerate disks.
        </p>
        <Button variant="secondary" size="sm" className="mt-4" onClick={fetchDrives}>
          Refresh Drives
        </Button>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
      {drives.map((drive) => (
        <DriveCard
          key={drive.id}
          drive={drive}
          isSelected={drive.id === selectedDriveId}
          onSelect={selectDrive}
        />
      ))}
    </div>
  );
};
