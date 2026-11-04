-- DiskAtlas Database Migration: 0008_cleanup_audit_history.sql

-- 1. Cleanup Audit History Table
CREATE TABLE IF NOT EXISTS cleanup_audit_events (
    id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL,
    session_id TEXT,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    category TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    cleanup_method TEXT NOT NULL, -- 'recycle_bin', 'permanent_delete', 'skipped'
    status TEXT NOT NULL, -- 'success', 'failed', 'blocked_security', 'in_use', 'permission_denied', 'not_found'
    is_reversible INTEGER NOT NULL DEFAULT 1,
    error_message TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(session_id) REFERENCES scan_sessions(id) ON DELETE SET NULL
);

-- 2. Query Indexes
CREATE INDEX IF NOT EXISTS idx_cleanup_audit_batch ON cleanup_audit_events(batch_id);
CREATE INDEX IF NOT EXISTS idx_cleanup_audit_created ON cleanup_audit_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cleanup_audit_session ON cleanup_audit_events(session_id);
CREATE INDEX IF NOT EXISTS idx_cleanup_audit_status ON cleanup_audit_events(status);
