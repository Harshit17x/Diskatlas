-- DiskAtlas Database Migration: 0011_normalize_summaries_and_indexes.sql
--
-- Ensures `category_summaries` uses the canonical 16-category classifier taxonomy
-- consistently across sessions (repairing any sessions created between 0010 and
-- the engine update that still used the legacy 9-category extension labels).

DELETE FROM category_summaries
WHERE session_id IN (
    SELECT DISTINCT session_id
    FROM category_summaries
    WHERE category IN ('code', 'executables', 'other', 'system')
);

INSERT OR REPLACE INTO category_summaries (session_id, category, file_count, total_bytes)
SELECT
    s.id,
    COALESCE(NULLIF(f.category, ''), 'unknown') AS cat,
    COUNT(*) AS file_count,
    COALESCE(SUM(f.size_bytes), 0) AS total_bytes
FROM scan_sessions s
JOIN files f ON f.session_id = s.id
WHERE s.status IN ('completed', 'completed_with_errors', 'completedWithErrors')
  AND s.id NOT IN (SELECT DISTINCT session_id FROM category_summaries)
GROUP BY s.id, COALESCE(NULLIF(f.category, ''), 'unknown');
