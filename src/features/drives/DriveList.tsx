import React from 'react';
import { useDriveStore } from '../../stores/driveStore';
import { DriveCard } from './DriveCard';
import { HardDrive, AlertCircle } from 'lucide-react';

export const DriveList: React.FC = () => {
  const { drives, selectedDriveId, selectDrive, isLoading, error } = useDriveStore();

  if (isLoading && drives.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-slate-500">
        <div className="w-8 h-8 border-2 border-sky-500 border-t-transparent rounded-full animate-spin mb-3"></div>
        <p className="text-sm">Detecting attached storage volumes...</p>
      </div>
    );
  }

  if (error && drives.length === 0) {
    return (
      <div className="p-6 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-300 flex items-start gap-3">
        <AlertCircle className="w-5 h-5 flex-shrink-0 mt-0.5 text-rose-400" />
        <div>
          <h4 className="font-semibold text-sm">Failed to detect drives</h4>
          <p className="text-xs text-rose-300/80 mt-1">{error}</p>
        </div>
      </div>
    );
  }

  if (drives.length === 0) {
    return (
      <div className="text-center py-16 border border-dashed border-slate-800 rounded-xl">
        <HardDrive className="w-10 h-10 text-slate-600 mx-auto mb-3" />
        <h3 className="text-sm font-semibold text-slate-300">No storage volumes found</h3>
        <p className="text-xs text-slate-500 mt-1">
          Verify drive permissions or refresh the drive list.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
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

