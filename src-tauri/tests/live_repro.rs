// TEMPORARY reproduction harness for the DiskAtlas review. Delete after review.
// Proves the SQLite GROUP BY <alias> shadowing bug in the engine's
// category_summaries aggregation, using the live production dataset read-only.
use rusqlite::Connection;

#[test]
fn repro_category_summaries_group_by_shadowing() {
    let path = std::env::var("HOME").unwrap()
        + "/Library/Application Support/com.diskatlas.app/diskatlas.db";
    let conn = Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();

    let sid: String = conn
        .query_row(
            "SELECT id FROM scan_sessions WHERE status IN ('completed_with_errors','completed') ORDER BY started_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    println!("session = {}", sid);

    // EXACT query from engine.rs:824-843 (category_summaries aggregation)
    let engine_rows: Vec<(String, i64, i64)> = {
        let mut stmt = conn
            .prepare(
                "SELECT
                    CASE
                        WHEN is_system = 1 THEN 'system'
                        WHEN extension IN ('rs','ts','tsx','js','jsx','py','go','c','cpp','h','hpp','java','cs','swift','kt','html','css','scss','json','yaml','yml','toml','xml','sql','sh','bash','zsh','bat','ps1') THEN 'code'
                        WHEN extension IN ('pdf','doc','docx','xls','xlsx','ppt','pptx','txt','md','csv','tsv','rtf','epub','pages','numbers','key') THEN 'documents'
                        WHEN extension IN ('png','jpg','jpeg','gif','svg','webp','ico','bmp','tiff','psd','ai','raw','heic') THEN 'images'
                        WHEN extension IN ('mp4','mkv','mov','avi','wmv','flv','webm','m4v','mpg','mpeg') THEN 'videos'
                        WHEN extension IN ('mp3','wav','flac','aac','ogg','m4a','wma','aiff') THEN 'audio'
                        WHEN extension IN ('zip','tar','gz','7z','rar','bz2','xz','iso','dmg','pkg','tgz') THEN 'archives'
                        WHEN extension IN ('exe','dll','msi','dylib','so','app','bin','elf') THEN 'executables'
                        ELSE 'other'
                    END AS category,
                    COUNT(*),
                    COALESCE(SUM(size_bytes), 0)
                 FROM files
                 WHERE session_id = ?1
                 GROUP BY category",
            )
            .unwrap();
        let rows = stmt
            .query_map([&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        rows
    };

    println!("\n=== ENGINE-STYLE QUERY RESULT ({} rows) ===", engine_rows.len());
    let mut seen = std::collections::BTreeMap::new();
    for (c, n, b) in &engine_rows {
        println!("  category={:<12} file_count={:<9} total_bytes={}", c, n, b);
        *seen.entry(c.clone()).or_insert(0i64) += n;
    }
    println!("  (repeated labels mean INSERT OR REPLACE collapses them: UNIQUE(session_id,category))");
    println!("  label -> summed file_count across emitted rows:");
    for (c, n) in &seen {
        println!("     {:<12} {}", c, n);
    }

    // What is ACTUALLY stored in category_summaries for this session
    let stored: Vec<(String, i64, i64)> = {
        let mut stmt = conn
            .prepare("SELECT category, file_count, total_bytes FROM category_summaries WHERE session_id=?1 ORDER BY total_bytes DESC")
            .unwrap();
        stmt.query_map([&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect()
    };
    println!("\n=== ACTUALLY STORED IN category_summaries ({} rows) ===", stored.len());
    for s in &stored {
        println!("  {:?}", s);
    }
    let total_files: i64 = conn
        .query_row("SELECT total_files FROM scan_sessions WHERE id=?1", [&sid], |r| r.get(0))
        .unwrap();
    println!("  session.total_files = {}", total_files);

    // The classifier's correct query (classification_summaries) for comparison
    let class_rows: Vec<(String, i64, i64)> = {
        let mut stmt = conn
            .prepare("SELECT category, SUM(file_count), SUM(total_bytes) FROM classification_summaries WHERE session_id=?1 GROUP BY category ORDER BY 3 DESC")
            .unwrap();
        stmt.query_map([&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect()
    };
    println!("\n=== classification_summaries (correct per-category breakdown, {} cats) ===", class_rows.len());
    for c in class_rows.iter().take(6) {
        println!("  {:?}", c);
    }

    // Chart source check: visualization reads classification_summaries WHERE subcategory IS NULL
    let chart_rows: Vec<(String, i64, i64)> = {
        let mut stmt = conn
            .prepare("SELECT category, file_count, total_bytes FROM classification_summaries WHERE session_id=?1 AND subcategory IS NULL ORDER BY total_bytes DESC")
            .unwrap();
        stmt.query_map([&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect()
    };
    println!("\n=== CHART SOURCE: classification_summaries WHERE subcategory IS NULL ({} rows) ===", chart_rows.len());
    for c in chart_rows.iter().take(6) {
        println!("  {:?}", c);
    }
    println!("  -> if 0, the analytics category chart falls back to a raw files-table GROUP BY");

    // developer pipeline scale check
    let markers: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT parent_path) FROM files WHERE session_id=?1 AND name IN ('package.json','Cargo.toml','pyproject.toml','setup.py','pom.xml','build.gradle','build.gradle.kts','CMakeLists.txt','.git')",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    let dev_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM developer_projects", [], |r| r.get(0))
        .unwrap();
    println!("\n=== DEVELOPER PIPELINE ===");
    println!("  distinct project roots to analyze = {}", markers);
    println!("  developer_projects rows actually stored = {}", dev_rows);
    println!("  -> each root triggers a full `path LIKE 'root/%'` scan of the files table");
}
