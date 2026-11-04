-- DiskAtlas Database Migration: 0002_files_and_directories.sql

-- 1. Scanned Files
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    extension TEXT,
    parent_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    allocated_bytes INTEGER,
    created_at INTEGER,
    modified_at INTEGER,
    is_hidden INTEGER NOT NULL DEFAULT 0,
    is_system INTEGER NOT NULL DEFAULT 0,
    is_symlink INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 2. Scanned Directories & Aggregations
CREATE TABLE IF NOT EXISTS directories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    parent_path TEXT,
    file_count INTEGER NOT NULL DEFAULT 0,
    dir_count INTEGER NOT NULL DEFAULT 0,
    total_size_bytes INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER,
    modified_at INTEGER,
    is_hidden INTEGER NOT NULL DEFAULT 0,
    is_system INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 3. Scan Errors & Inaccessible Paths Audit
CREATE TABLE IF NOT EXISTS scan_errors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    error_type TEXT NOT NULL,
    message TEXT NOT NULL,
    occurred_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 4. High-Throughput Query Indexes
CREATE INDEX IF NOT EXISTS idx_files_session_parent ON files(session_id, parent_path);
CREATE INDEX IF NOT EXISTS idx_files_session_size ON files(session_id, size_bytes DESC);
CREATE INDEX IF NOT EXISTS idx_files_session_ext ON files(session_id, extension);
CREATE INDEX IF NOT EXISTS idx_dirs_session_parent ON directories(session_id, parent_path);
CREATE INDEX IF NOT EXISTS idx_dirs_session_size ON directories(session_id, total_size_bytes DESC);
CREATE INDEX IF NOT EXISTS idx_scan_errors_session ON scan_errors(session_id);

