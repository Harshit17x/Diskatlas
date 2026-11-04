-- DiskAtlas Database Migration: 0007_storage_intelligence.sql

-- 1. Storage Recommendations and Intelligence Findings
CREATE TABLE IF NOT EXISTS storage_recommendations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    category TEXT NOT NULL,
    rec_group TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    reason TEXT NOT NULL,
    confidence REAL NOT NULL,
    recoverable_bytes INTEGER NOT NULL,
    expected_consequences TEXT NOT NULL,
    recommended_action TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- 2. High-performance Query Indexes
CREATE INDEX IF NOT EXISTS idx_recommendations_session ON storage_recommendations(session_id);
CREATE INDEX IF NOT EXISTS idx_recommendations_group ON storage_recommendations(session_id, rec_group);
CREATE INDEX IF NOT EXISTS idx_recommendations_risk ON storage_recommendations(session_id, risk_level);
CREATE INDEX IF NOT EXISTS idx_recommendations_size ON storage_recommendations(session_id, size_bytes DESC);

