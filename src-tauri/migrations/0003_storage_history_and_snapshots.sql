-- DiskAtlas Database Migration: 0003_storage_history_and_snapshots.sql

-- 1. Storage Snapshots (Point-in-time storage state)
CREATE TABLE IF NOT EXISTS storage_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    drive_id TEXT NOT NULL,
    label TEXT NOT NULL,
    root_path TEXT NOT NULL,
    total_files INTEGER NOT NULL DEFAULT 0,
    total_directories INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE SET NULL,
    FOREIGN KEY(drive_id) REFERENCES drives(id) ON DELETE CASCADE
);

-- 2. Aggregated Category Summaries (Computed per scan session)
CREATE TABLE IF NOT EXISTS category_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    category TEXT NOT NULL,
    file_count INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE,
    UNIQUE(session_id, category)
);

-- 3. Differential Comparison & History Query Indexes
CREATE INDEX IF NOT EXISTS idx_snapshots_drive ON storage_snapshots(drive_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_snapshots_session ON storage_snapshots(session_id);
CREATE INDEX IF NOT EXISTS idx_category_summaries_session ON category_summaries(session_id);
CREATE INDEX IF NOT EXISTS idx_files_session_path ON files(session_id, path);
CREATE INDEX IF NOT EXISTS idx_dirs_session_path ON directories(session_id, path);
