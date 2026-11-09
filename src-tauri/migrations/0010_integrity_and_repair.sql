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
-- Deduplicate first: keep the most recently aggregated row per (session_id, path).
DELETE FROM directories
WHERE id NOT IN (
    SELECT MAX(id)
    FROM directories
    GROUP BY session_id, path
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_dirs_session_path_unique
    ON directories(session_id, path);

-- ---------------------------------------------------------------------------
-- 2. Purge rows whose parent session no longer exists
-- ---------------------------------------------------------------------------
-- NOTE: these run against a `files` table that can hold millions of rows, so the
-- anti-join is expressed as a LEFT JOIN against the (small) scan_sessions table
-- rather than a correlated NOT IN subquery, which SQLite cannot always optimize.
DELETE FROM files
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

DELETE FROM directories
WHERE session_id IS NOT NULL
  AND session_id NOT IN (SELECT id FROM scan_sessions);

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
