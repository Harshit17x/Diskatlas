-- DiskAtlas Database Migration: 0012_compact_indexes_and_retention.sql
--
-- Removes redundant and unused multi-column indexes on `directories` and `files`
-- that multiplied per-scan SQLite storage footprint without accelerating active queries:
-- 1. `idx_dirs_session_path` duplicates `idx_dirs_session_path_unique` on `(session_id, path)`.
-- 2. `idx_files_session_hidden` cannot be used due to OR/LIKE/instr predicates.
-- 3. `idx_files_session_ext` is bypassed by LOWER(extension) and size-first predicates.
-- 4. `idx_files_session_subcat` is never queried on `files` (`classification_summaries` serves it).
-- 5. `idx_files_session_app` unnecessarily indexes millions of NULL app_name rows.

DROP INDEX IF EXISTS idx_dirs_session_path;
DROP INDEX IF EXISTS idx_files_session_hidden;
DROP INDEX IF EXISTS idx_files_session_ext;
DROP INDEX IF EXISTS idx_files_session_subcat;
DROP INDEX IF EXISTS idx_files_session_app;
