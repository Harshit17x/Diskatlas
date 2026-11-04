-- DiskAtlas Database Migration: 0005_developer_storage.sql

-- 1. Detected Developer Projects
CREATE TABLE IF NOT EXISTS developer_projects (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    dependency_bytes INTEGER NOT NULL DEFAULT 0,
    cache_bytes INTEGER NOT NULL DEFAULT 0,
    build_output_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    file_count INTEGER NOT NULL DEFAULT 0,
    last_modified_at INTEGER,
    confidence REAL NOT NULL DEFAULT 0.9,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 2. Aggregated Developer Ecosystem Summaries
CREATE TABLE IF NOT EXISTS developer_ecosystem_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    ecosystem TEXT NOT NULL,
    project_count INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    dependency_bytes INTEGER NOT NULL DEFAULT 0,
    cache_bytes INTEGER NOT NULL DEFAULT 0,
    build_bytes INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 3. Query Indexes
CREATE INDEX IF NOT EXISTS idx_dev_projects_session ON developer_projects(session_id, total_bytes DESC);
CREATE INDEX IF NOT EXISTS idx_dev_ecosystems_session ON developer_ecosystem_summaries(session_id);

