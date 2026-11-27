-- DiskAtlas Database Migration: 0010_integrity_and_repair.sql
--
-- Addresses three integrity problems found during the July 2026 codebase review:
--
-- 1. `directories` had no uniqueness constraint on (session_id, path). The scanner writes
--    directories in multiple batches (streaming flush + final rollup flush), so the same
--    path could be inserted twice. The visualization tree then returned duplicate nodes and
--    the root row could not be upserted. A unique index makes the write idempotent.
--
-- 2. Orphaned rows: `files` / `directories` rows whose parent session no longer exists
--    (because `scan_sessions` cascades on delete of a drive) were left behind, inflating
--    the database and skewing aggregate queries.
--
-- 3. `category_summaries` rows written by the pre-fix engine were corrupted by a SQLite
--    name-shadowing bug (GROUP BY bound to the `files.category` column instead of the
--    CASE alias), collapsing each session's breakdown into a single mislabeled row.
--    Those rows are recomputed here from the canonical `files.category` taxonomy.

-- ---------------------------------------------------------------------------
-- 1. Uniqueness for directories
-- ---------------------------------------------------------------------------
-- Deduplicate directories only for (session_id, path) keys that actually appear > 1 time.
DELETE FROM directories
WHERE id IN (
    SELECT d.id
    FROM directories d
    JOIN (
        SELECT session_id, path, MAX(id) AS keep_id
        FROM directories
        GROUP BY session_id, path
        HAVING COUNT(*) > 1
    ) dup ON d.session_id = dup.session_id AND d.path = dup.path AND d.id != dup.keep_id
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_dirs_session_path_unique
    ON directories(session_id, path);

-- ---------------------------------------------------------------------------
-- 2. Purge rows whose parent session no longer exists (index skip-scan on files/dirs)
-- ---------------------------------------------------------------------------
DELETE FROM files
WHERE session_id IN (
    WITH RECURSIVE sids(sid) AS (
        SELECT MIN(session_id) FROM files
        UNION ALL
        SELECT (SELECT MIN(session_id) FROM files WHERE session_id > sids.sid)
        FROM sids WHERE sids.sid IS NOT NULL
    )
    SELECT sid FROM sids
    WHERE sid IS NOT NULL
      AND NOT EXISTS (SELECT 1 FROM scan_sessions WHERE id = sid)
);

DELETE FROM directories
WHERE session_id IN (
    WITH RECURSIVE sids(sid) AS (
        SELECT MIN(session_id) FROM directories
        UNION ALL
        SELECT (SELECT MIN(session_id) FROM directories WHERE session_id > sids.sid)
        FROM sids WHERE sids.sid IS NOT NULL
    )
    SELECT sid FROM sids
    WHERE sid IS NOT NULL
      AND NOT EXISTS (SELECT 1 FROM scan_sessions WHERE id = sid)
);

DELETE FROM scan_errors
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

DELETE FROM classification_summaries
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

DELETE FROM category_summaries
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

DELETE FROM developer_projects
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

DELETE FROM developer_ecosystem_summaries
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

DELETE FROM storage_recommendations
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

-- ---------------------------------------------------------------------------
-- 3. Rebuild corrupted category_summaries from the canonical taxonomy
-- ---------------------------------------------------------------------------
DELETE FROM category_summaries;

INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
SELECT
    session_id,
    COALESCE(NULLIF(category, ''), 'unknown') AS category,
    COUNT(*) AS file_count,
    COALESCE(SUM(size_bytes), 0) AS total_bytes
FROM files
GROUP BY session_id, COALESCE(NULLIF(category, ''), 'unknown');

-- ---------------------------------------------------------------------------
-- 4. Indexes supporting the storage-history comparison queries
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_category_summaries_unique
    ON category_summaries(session_id, category);

CREATE INDEX IF NOT EXISTS idx_class_summaries_category
    ON classification_summaries(session_id, category);
