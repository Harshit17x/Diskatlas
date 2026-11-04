-- DiskAtlas Database Initial Migration: 0001_initial_schema.sql

-- 1. Migration History
CREATE TABLE IF NOT EXISTS _migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Discovered Storage Drives & Partitions
CREATE TABLE IF NOT EXISTS drives (
    id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    mount_point TEXT NOT NULL,
    filesystem TEXT NOT NULL,
    total_bytes INTEGER NOT NULL,
    available_bytes INTEGER NOT NULL,
    is_removable INTEGER NOT NULL DEFAULT 0,
    is_system INTEGER NOT NULL DEFAULT 0,
    last_scanned_at DATETIME
);

-- 3. Scan Sessions
CREATE TABLE IF NOT EXISTS scan_sessions (
    id TEXT PRIMARY KEY,
    drive_id TEXT NOT NULL,
    root_path TEXT NOT NULL,
    status TEXT NOT NULL,
    total_files INTEGER DEFAULT 0,
    total_directories INTEGER DEFAULT 0,
    total_bytes INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY(drive_id) REFERENCES drives(id) ON DELETE CASCADE
);

-- 4. Application Settings
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 5. Indexes
CREATE INDEX IF NOT EXISTS idx_scan_sessions_drive ON scan_sessions(drive_id);
CREATE INDEX IF NOT EXISTS idx_scan_sessions_status ON scan_sessions(status);

