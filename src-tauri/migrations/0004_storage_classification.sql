-- DiskAtlas Database Migration: 0004_storage_classification.sql

-- 1. User and System Category Overrides
CREATE TABLE IF NOT EXISTS category_overrides (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern TEXT UNIQUE NOT NULL,
    category TEXT NOT NULL,
    subcategory TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 2. Add Classification Columns to Files Table
ALTER TABLE files ADD COLUMN category TEXT DEFAULT 'unknown';
ALTER TABLE files ADD COLUMN subcategory TEXT;
ALTER TABLE files ADD COLUMN app_name TEXT;
ALTER TABLE files ADD COLUMN confidence REAL DEFAULT 0.5;

-- 3. Detailed Classification Summaries (Category & Subcategory Breakdown)
CREATE TABLE IF NOT EXISTS classification_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    category TEXT NOT NULL,
    subcategory TEXT,
    app_name TEXT,
    file_count INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    avg_confidence REAL NOT NULL DEFAULT 0.5,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 4. High-Performance Query Indexes
CREATE INDEX IF NOT EXISTS idx_files_session_cat ON files(session_id, category);
CREATE INDEX IF NOT EXISTS idx_files_session_subcat ON files(session_id, category, subcategory);
CREATE INDEX IF NOT EXISTS idx_files_session_app ON files(session_id, app_name);
CREATE INDEX IF NOT EXISTS idx_class_summaries_session ON classification_summaries(session_id);
