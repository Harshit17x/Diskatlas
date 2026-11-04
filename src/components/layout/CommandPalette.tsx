import React, { useState, useEffect, useRef } from 'react';
import {
  LayoutDashboard,
  HardDrive,
  PieChart,
  Terminal,
  EyeOff,
  Box,
  Copy,
  Sparkles,
  History,
  Settings,
  RefreshCw,
  Search,
} from 'lucide-react';
import { useUIStore } from '../../stores/uiStore';
import { useDriveStore } from '../../stores/driveStore';

interface CommandItem {
  id: string;
  title: string;
  category: 'Navigation' | 'Actions';
  icon: React.ReactNode;
  shortcut?: string;
  onSelect: () => void;
}

export const CommandPalette: React.FC = () => {
  const { commandPaletteOpen, setCommandPaletteOpen, setActiveTab } = useUIStore();
  const { fetchDrives, fetchSystemInfo, fetchDbHealth } = useDriveStore();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: CommandItem[] = [
    {
      id: 'nav-dashboard',
      title: 'Go to Dashboard',
      category: 'Navigation',
      icon: <LayoutDashboard className="w-4 h-4" />,
      shortcut: 'G D',
      onSelect: () => setActiveTab('dashboard'),
    },
    {
      id: 'nav-explorer',
      title: 'Go to Drive Explorer',
      category: 'Navigation',
      icon: <HardDrive className="w-4 h-4" />,
      shortcut: 'G E',
      onSelect: () => setActiveTab('explorer'),
    },
    {
      id: 'nav-analysis',
      title: 'Go to Storage Analysis',
      category: 'Navigation',
      icon: <PieChart className="w-4 h-4" />,
      shortcut: 'G A',
      onSelect: () => setActiveTab('analysis'),
    },
    {
      id: 'nav-developer',
      title: 'Go to Developer Storage',
      category: 'Navigation',
      icon: <Terminal className="w-4 h-4" />,
      onSelect: () => setActiveTab('developer'),
    },
    {
      id: 'nav-hidden',
      title: 'Go to Hidden Files',
      category: 'Navigation',
      icon: <EyeOff className="w-4 h-4" />,
      onSelect: () => setActiveTab('hidden'),
    },
    {
      id: 'nav-applications',
      title: 'Go to Applications',
      category: 'Navigation',
      icon: <Box className="w-4 h-4" />,
      onSelect: () => setActiveTab('applications'),
    },
    {
      id: 'nav-duplicates',
      title: 'Go to Duplicate Finder',
      category: 'Navigation',
      icon: <Copy className="w-4 h-4" />,
      onSelect: () => setActiveTab('duplicates'),
    },
    {
      id: 'nav-cleanup',
      title: 'Go to Cleanup Center',
      category: 'Navigation',
      icon: <Sparkles className="w-4 h-4" />,
      onSelect: () => setActiveTab('cleanup'),
    },
    {
      id: 'nav-history',
      title: 'Go to Storage History',
      category: 'Navigation',
      icon: <History className="w-4 h-4" />,
      onSelect: () => setActiveTab('history'),
    },
    {
      id: 'nav-settings',
      title: 'Go to Settings',
      category: 'Navigation',
      icon: <Settings className="w-4 h-4" />,
      shortcut: 'G S',
      onSelect: () => setActiveTab('settings'),
    },
    {
      id: 'action-refresh',
      title: 'Refresh Discovered Drives',
      category: 'Actions',
      icon: <RefreshCw className="w-4 h-4" />,
      shortcut: 'R',
      onSelect: () => {
        fetchDrives();
        fetchSystemInfo();
        fetchDbHealth();
      },
    },
  ];

  const filtered = commands.filter((cmd) =>
    cmd.title.toLowerCase().includes(query.toLowerCase())
  );

  useEffect(() => {
    if (commandPaletteOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [commandPaletteOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Toggle Command Palette with Cmd+K or Ctrl+K
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setCommandPaletteOpen(!commandPaletteOpen);
      }

      if (!commandPaletteOpen) return;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev + 1) % (filtered.length || 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev - 1 + (filtered.length || 1)) % (filtered.length || 1));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (filtered[selectedIndex]) {
          filtered[selectedIndex].onSelect();
          setCommandPaletteOpen(false);
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        setCommandPaletteOpen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [commandPaletteOpen, filtered, selectedIndex, setCommandPaletteOpen]);

  if (!commandPaletteOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-24 p-4 bg-black/60"
      onClick={() => setCommandPaletteOpen(false)}
    >
      <div
        className="w-full max-w-xl bg-panel border border-border rounded shadow-2xl overflow-hidden flex flex-col text-primary animate-in fade-in zoom-in-95 duration-100"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search Input Bar */}
        <div className="flex items-center px-4 py-3 border-b border-border bg-[#14161A]">
          <Search className="w-4 h-4 text-muted mr-3 flex-shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            placeholder="Search commands, views, or tools..."
            className="w-full bg-transparent text-sm text-primary placeholder-muted focus:outline-none"
          />
          <span className="text-[10px] font-mono text-muted px-1.5 py-0.5 rounded border border-border bg-panel">
            ESC
          </span>
        </div>

        {/* Results List */}
        <div className="max-h-80 overflow-y-auto p-2 space-y-1">
          {filtered.length > 0 ? (
            filtered.map((cmd, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <button
                  key={cmd.id}
                  onClick={() => {
                    cmd.onSelect();
                    setCommandPaletteOpen(false);
                  }}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded text-xs transition-colors ${
                    isSelected ? 'bg-accent text-white' : 'text-primary hover:bg-elevated'
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <span className={isSelected ? 'text-white' : 'text-secondary'}>
                      {cmd.icon}
                    </span>
                    <span className="font-medium">{cmd.title}</span>
                    <span
                      className={`text-[10px] px-1.5 py-0.5 rounded ${
                        isSelected ? 'bg-white/20 text-white' : 'bg-elevated text-muted'
                      }`}
                    >
                      {cmd.category}
                    </span>
                  </div>

                  {cmd.shortcut && (
                    <span
                      className={`text-[10px] font-mono px-1.5 py-0.5 rounded ${
                        isSelected ? 'bg-white/20 text-white' : 'bg-[#14161A] text-muted border border-border'
                      }`}
                    >
                      {cmd.shortcut}
                    </span>
                  )}
                </button>
              );
            })
          ) : (
            <div className="py-8 text-center text-xs text-muted">
              No matching commands or routes for &ldquo;{query}&rdquo;
            </div>
          )}
        </div>

        {/* Footer info */}
        <div className="px-4 py-2 border-t border-border bg-[#14161A] flex items-center justify-between text-[11px] text-muted">
          <span>Use ↑ ↓ to navigate</span>
          <span>Press Enter to select</span>
        </div>
      </div>
    </div>
  );
};
