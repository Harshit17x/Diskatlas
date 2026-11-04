-- DiskAtlas Database Migration: 0006_advanced_analysis.sql

-- 1. Cached Cryptographic File Hashes (Prevents re-hashing unchanged files during staged duplicate detection)
CREATE TABLE IF NOT EXISTS file_hashes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT UNIQUE NOT NULL,
    size_bytes INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Indexes for accelerated duplicate candidate grouping
CREATE INDEX IF NOT EXISTS idx_file_hashes_size ON file_hashes(size_bytes);
CREATE INDEX IF NOT EXISTS idx_file_hashes_sha ON file_hashes(sha256);
CREATE INDEX IF NOT EXISTS idx_files_session_hidden ON files(session_id, is_hidden, is_system);
CREATE INDEX IF NOT EXISTS idx_files_session_mod ON files(session_id, modified_at);

