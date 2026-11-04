import React, { useEffect } from 'react';
import { Header } from '../components/layout/Header';
import { Sidebar } from '../components/layout/Sidebar';
import { StatusBar } from '../components/layout/StatusBar';
import { DashboardView } from '../features/dashboard/DashboardView';
import { DriveList } from '../features/drives/DriveList';
import { useUIStore } from '../stores/uiStore';
import { useDriveStore } from '../stores/driveStore';
import { HardDrive, AlertCircle } from 'lucide-react';

export const App: React.FC = () => {
  const { activeTab } = useUIStore();
  const { fetchDrives, fetchSystemInfo, fetchDbHealth } = useDriveStore();

  useEffect(() => {
    fetchDrives();
    fetchSystemInfo();
    fetchDbHealth();
  }, [fetchDrives, fetchSystemInfo, fetchDbHealth]);

  const renderContent = () => {
    switch (activeTab) {
      case 'dashboard':
        return <DashboardView />;
      case 'drives':
        return (
          <div className="space-y-4">
            <div>
              <h2 className="text-lg font-semibold text-slate-100">All Connected Volumes</h2>
              <p className="text-xs text-slate-400">View and inspect individual partitions</p>
            </div>
            <DriveList />
          </div>
        );
      case 'large_files':
      case 'duplicates':
      case 'recommendations':
      case 'settings':
        return (
          <div className="flex flex-col items-center justify-center py-24 text-center">
            <div className="w-12 h-12 rounded-xl bg-slate-900 border border-slate-800 flex items-center justify-center mb-3">
              <HardDrive className="w-6 h-6 text-slate-500" />
            </div>
            <h3 className="text-sm font-semibold text-slate-300">
              {activeTab.replace('_', ' ').toUpperCase()} Module
            </h3>
            <p className="text-xs text-slate-500 max-w-sm mt-1">
              Scheduled for implementation in Phase 2 through Phase 5 according to the roadmap.
            </p>
          </div>
        );
      default:
        return (
          <div className="p-4 text-center text-slate-500">
            <AlertCircle className="w-6 h-6 mx-auto mb-2 text-amber-500" />
            Unknown view requested.
          </div>
        );
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-slate-950 text-slate-100">
      <Header />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main className="flex-1 overflow-y-auto p-6 bg-slate-950">
          <div className="max-w-7xl mx-auto">{renderContent()}</div>
        </main>
      </div>
      <StatusBar />
    </div>
  );
};

