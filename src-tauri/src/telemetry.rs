use rusqlite::{params, Connection, Result};
use serde::Serialize;

const ALLOWED_EVENTS: [&str; 3] = ["recording_done", "query_asked", "reminder_clicked"];

pub fn log_event(conn: &Connection, event: &str) -> Result<()> {
    if !ALLOWED_EVENTS.contains(&event) {
        return Err(rusqlite::Error::InvalidParameterName(format!("unknown event {event}")));
    }
    conn.execute("INSERT INTO usage_events (event) VALUES (?1)", params![event])?;
    Ok(())
}

#[derive(Debug, Serialize, Default)]
pub struct UsageStats {
    pub recordings: i64,
    pub queries: i64,
    pub reminder_clicks: i64,
    pub last_7d_active_days: i64,
}

pub fn usage_stats(conn: &Connection) -> Result<UsageStats> {
    let count = |event: &str| -> Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM usage_events WHERE event = ?1",
            params![event],
            |r| r.get(0),
        )
    };
    Ok(UsageStats {
        recordings: count("recording_done")?,
        queries: count("query_asked")?,
        reminder_clicks: count("reminder_clicked")?,
        last_7d_active_days: conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM usage_events
             WHERE date(created_at) >= date('now', '-7 days')",
            [], |r| r.get(0),
        )?,
    })
}
