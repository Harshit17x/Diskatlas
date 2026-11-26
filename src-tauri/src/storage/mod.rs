pub mod connection;
pub mod migrator;

pub use connection::DbPool;

use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::Connection;
use crate::error::{AppError, Result};

/// Resolves and validates a scan session ID for read/analysis views.
///
/// - When `requested` is `Some(id)` (non-empty and not `"starting"`), validates that the
///   session exists in `scan_sessions` and returns `AppError::NotFound` if missing.
/// - Otherwise resolves the newest usable session, preferring completed sessions with
///   `total_files > 0`, and falling back to non-empty `interrupted`/`cancelled` sessions
///   only when no completed session exists.
pub fn resolve_session_id(conn: &Connection, requested: Option<&str>) -> Result<String> {
    if let Some(id) = requested {
        let trimmed = id.trim();
        if !trimmed.is_empty() && trimmed != "starting" {
            let exists: Option<String> = conn
                .query_row(
                    "SELECT id FROM scan_sessions WHERE id = ?1",
                    [trimmed],
                    |row| row.get(0),
                )
                .ok();
            return match exists {
                Some(found) => Ok(found),
                None => Err(AppError::NotFound(format!(
                    "Scan session '{}' not found",
                    trimmed
                ))),
            };
        }
    }

    let completed: Option<String> = conn
        .query_row(
            "SELECT id FROM scan_sessions
             WHERE status IN ('completed', 'completedWithErrors', 'completed_with_errors')
             ORDER BY (CASE WHEN COALESCE(total_files, 0) > 0 THEN 0 ELSE 1 END) ASC,
                      started_at DESC,
                      rowid DESC
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = completed {
        return Ok(id);
    }

    let fallback: Option<String> = conn
        .query_row(
            "SELECT id FROM scan_sessions
             WHERE COALESCE(total_files, 0) > 0
               AND status IN ('interrupted', 'cancelled')
             ORDER BY started_at DESC, rowid DESC
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    match fallback {
        Some(id) => Ok(id),
        None => Err(AppError::NotFound(
            "No completed scan sessions found in database".to_string(),
        )),
    }
}

/// Escapes SQLite `LIKE` wildcard characters (`\`, `%`, `_`) for use with `ESCAPE '\'`.
pub fn escape_like_pattern(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Builds an escaped SQLite `LIKE` prefix pattern matching direct and nested descendants of `path`.
pub fn child_like_pattern(path: &str) -> String {
    let sep = if path.contains('\\') && !path.contains('/') {
        '\\'
    } else {
        '/'
    };
    let prefix = if path == "/" {
        "/".to_string()
    } else if path.len() == 3 && path.ends_with(":\\") {
        path.to_string()
    } else {
        let trimmed = path.trim_end_matches(&['/', '\\'][..]);
        format!("{}{}", trimmed, sep)
    };
    format!("{}%", escape_like_pattern(&prefix))
}

/// Formats the current system time as an accurate UTC ISO-8601 timestamp (`YYYY-MM-DDTHH:MM:SSZ`).
pub fn utc_iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_unix_utc_iso(secs)
}

/// Converts a Unix timestamp (seconds since 1970-01-01T00:00:00Z) into UTC ISO-8601.
pub fn format_unix_utc_iso(unix_secs: i64) -> String {
    let days = unix_secs.div_euclid(86_400);
    let secs_of_day = unix_secs.rem_euclid(86_400);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;

    // Howard Hinnant civil-from-days algorithm
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hour, minute, second
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrator::Migrator;

    #[test]
    fn test_resolve_session_id_prefers_completed_over_newer_cancelled() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrator::run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
             VALUES ('D1', 'Drive', '/', 'apfs', 1000, 500)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, started_at)
             VALUES
             ('sess-completed', 'D1', '/', 'completed_with_errors', 42, '2026-11-05 10:00:00'),
             ('sess-cancelled', 'D1', '/', 'cancelled', 0, '2026-11-06 10:00:00')",
            [],
        )
        .unwrap();

        let resolved = resolve_session_id(&conn, None).unwrap();
        assert_eq!(resolved, "sess-completed");
        assert!(resolve_session_id(&conn, Some("missing-id")).is_err());
        assert_eq!(
            resolve_session_id(&conn, Some("starting")).unwrap(),
            "sess-completed"
        );
    }

    #[test]
    fn test_like_escaping_and_utc_iso() {
        assert_eq!(escape_like_pattern("a_b%c\\d"), "a\\_b\\%c\\\\d");
        assert_eq!(child_like_pattern("/tmp/a_b"), "/tmp/a\\_b/%");
        assert_eq!(child_like_pattern("/"), "/%");
        assert_eq!(child_like_pattern("C:\\work_1"), "C:\\\\work\\_1\\\\%");
        assert_eq!(child_like_pattern("C:\\"), "C:\\\\%");
        assert_eq!(format_unix_utc_iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix_utc_iso(1795651200), "2026-11-26T00:00:00Z");
        assert_eq!(format_unix_utc_iso(1795737600), "2026-11-27T00:00:00Z");
    }
}
