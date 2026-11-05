import React, { useEffect } from 'react';
import { Header } from '../components/layout/Header';
import { Sidebar } from '../components/layout/Sidebar';
import { StatusBar } from '../components/layout/StatusBar';
import { CommandPalette } from '../components/layout/CommandPalette';
import { DashboardView } from '../features/dashboard/DashboardView';
import { DriveExplorerView } from '../features/explorer/DriveExplorerView';
import { StorageAnalysisView } from '../features/analysis/StorageAnalysisView';
import { StorageIntelligenceView } from '../features/intelligence/StorageIntelligenceView';
import { DeveloperStorageView } from '../features/developer/DeveloperStorageView';
import { HiddenFilesView } from '../features/hidden/HiddenFilesView';
import { ApplicationsView } from '../features/applications/ApplicationsView';
import { DuplicateFinderView } from '../features/duplicates/DuplicateFinderView';
import { CleanupCenterView } from '../features/cleanup/CleanupCenterView';
import { StorageHistoryView } from '../features/history/StorageHistoryView';
import { SettingsView } from '../features/settings/SettingsView';
import { DriveDetailsModal } from '../features/drives/DriveDetailsModal';
import { ScanModal } from '../features/scanner/ScanModal';
import { useUIStore } from '../stores/uiStore';
import { useDriveStore } from '../stores/driveStore';
import { useScanStore } from '../stores/scanStore';

export const App: React.FC = () => {
  const { activeTab, setActiveTab, theme } = useUIStore();
  const { fetchDrives, fetchSystemInfo, fetchDbHealth } = useDriveStore();
  const { setupEventListeners } = useScanStore();

  // Sync theme class to document element
  useEffect(() => {
    if (theme === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme]);

  useEffect(() => {
    fetchDrives();
    fetchSystemInfo();
    fetchDbHealth();

    let cleanup: (() => void) | undefined;
    setupEventListeners().then((unlisten) => {
      cleanup = unlisten;
    });

    return () => {
      cleanup?.();
    };
  }, [fetchDrives, fetchSystemInfo, fetchDbHealth, setupEventListeners]);

  // Global keyboard shortcuts (Cmd+1 to Cmd+9)
  useEffect(() => {
    const handleKeyNav = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) {
        switch (e.key) {
          case '1':
            e.preventDefault();
            setActiveTab('dashboard');
            break;
          case '2':
            e.preventDefault();
            setActiveTab('explorer');
            break;
          case '3':
            e.preventDefault();
            setActiveTab('analysis');
            break;
          case '4':
            e.preventDefault();
            setActiveTab('developer');
            break;
          case '5':
            e.preventDefault();
            setActiveTab('hidden');
            break;
          case '6':
            e.preventDefault();
            setActiveTab('applications');
            break;
          case '7':
            e.preventDefault();
            setActiveTab('duplicates');
            break;
          case '8':
            e.preventDefault();
            setActiveTab('cleanup');
            break;
          case '9':
            e.preventDefault();
            setActiveTab('history');
            break;
          case ',':
            e.preventDefault();
            setActiveTab('settings');
            break;
        }
      }
    };

    window.addEventListener('keydown', handleKeyNav);
    return () => window.removeEventListener('keydown', handleKeyNav);
  }, [setActiveTab]);

  const renderContent = () => {
    switch (activeTab) {
      case 'dashboard':
        return <DashboardView />;
      case 'explorer':
        return <DriveExplorerView />;
      case 'analysis':
        return <StorageAnalysisView />;
      case 'intelligence':
        return <StorageIntelligenceView />;
      case 'developer':
        return <DeveloperStorageView />;
      case 'hidden':
        return <HiddenFilesView />;
      case 'applications':
        return <ApplicationsView />;
      case 'duplicates':
        return <DuplicateFinderView />;
      case 'cleanup':
        return <CleanupCenterView />;
      case 'history':
        return <StorageHistoryView />;
      case 'settings':
        return <SettingsView />;
      default:
        return <DashboardView />;
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-canvas text-primary">
      <CommandPalette />
      <DriveDetailsModal />
      <ScanModal />
      <Header />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <main className="flex-1 overflow-y-auto p-5 bg-canvas">
          <div className="max-w-6xl mx-auto">{renderContent()}</div>
        </main>
      </div>
      <StatusBar />
    </div>
  );
};
