DELETE FROM category_summaries
WHERE session_id IN (
    SELECT id FROM scan_sessions
    WHERE status IN ('completed', 'completed_with_errors')
);

INSERT OR REPLACE INTO category_summaries (session_id, category, file_count, total_bytes)
SELECT
    session_id,
    ext_category AS category,
    COUNT(*) as file_count,
    COALESCE(SUM(size_bytes), 0) as total_bytes
FROM (
    SELECT
        session_id,
        size_bytes,
        CASE
            WHEN is_system = 1 THEN 'system'
            WHEN extension IN ('rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'c', 'cpp', 'h', 'hpp', 'java', 'cs', 'swift', 'kt', 'html', 'css', 'scss', 'json', 'yaml', 'yml', 'toml', 'xml', 'sql', 'sh', 'bash', 'zsh', 'bat', 'ps1') THEN 'code'
            WHEN extension IN ('pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'txt', 'md', 'csv', 'tsv', 'rtf', 'epub', 'pages', 'numbers', 'key') THEN 'documents'
            WHEN extension IN ('png', 'jpg', 'jpeg', 'gif', 'svg', 'webp', 'ico', 'bmp', 'tiff', 'psd', 'ai', 'raw', 'heic') THEN 'images'
            WHEN extension IN ('mp4', 'mkv', 'mov', 'avi', 'wmv', 'flv', 'webm', 'm4v', 'mpg', 'mpeg') THEN 'videos'
            WHEN extension IN ('mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a', 'wma', 'aiff') THEN 'audio'
            WHEN extension IN ('zip', 'tar', 'gz', '7z', 'rar', 'bz2', 'xz', 'iso', 'dmg', 'pkg', 'tgz') THEN 'archives'
            WHEN extension IN ('exe', 'dll', 'msi', 'dylib', 'so', 'app', 'bin', 'elf') THEN 'executables'
            ELSE 'other'
        END AS ext_category
    FROM files
    WHERE session_id IN (
        SELECT id FROM scan_sessions
        WHERE status IN ('completed', 'completed_with_errors')
    )
)
GROUP BY session_id, ext_category;
