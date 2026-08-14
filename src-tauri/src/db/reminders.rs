use crate::memory::types::ReminderExtract;
use crate::timeparse::parse_due;
use chrono::NaiveDateTime;
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ReminderRow {
    pub id: i64,
    pub content: String,
    pub due_at: Option<String>,
    pub status: String,
    pub needs_time: bool,
    pub conversation_id: i64,
}

pub fn save_reminders(
    conn: &Connection,
    reminders: &[ReminderExtract],
    conversation_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    for r in reminders {
        if r.content.trim().is_empty() { continue; }
        let (due_at, needs_time) = match &r.due {
            Some(d) => {
                match parse_due(d, now) {
                    Some(dt) => (Some(dt.format("%Y-%m-%d %H:%M:%S").to_string()), false),
                    None => {
                        // 兜底：LLM 可能输出 ISO 日期
                        match NaiveDateTime::parse_from_str(d, "%Y-%m-%d %H:%M:%S")
                            .or_else(|_| NaiveDateTime::parse_from_str(d, "%Y-%m-%dT%H:%M:%S"))
                        {
                            Ok(dt) => (Some(dt.format("%Y-%m-%d %H:%M:%S").to_string()), false),
                            Err(_) => (Some(d.clone()), true),
                        }
                    }
                }
            }
            None => (None, true),
        };
        conn.execute(
            "INSERT INTO reminders (content, due_at, needs_time, conversation_id) VALUES (?1,?2,?3,?4)",
            params![r.content.trim(), due_at, needs_time as i64, conversation_id],
        )?;
        ids.push(conn.last_insert_rowid());
    }
    Ok(ids)
}

pub fn list_reminders(conn: &Connection) -> Result<Vec<ReminderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at, status, needs_time, conversation_id
         FROM reminders ORDER BY due_at IS NULL, due_at ASC, id DESC",
    )?;
    let rows = stmt.query_map([], map_row)?;
    rows.collect()
}

pub fn get_reminder(conn: &Connection, id: i64) -> Result<Option<ReminderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at, status, needs_time, conversation_id
         FROM reminders WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], map_row)?;
    rows.next().transpose()
}

pub fn set_status(conn: &Connection, id: i64, status: &str) -> Result<()> {
    match status {
        "pending" | "done" | "expired" => {}
        _ => return Err(rusqlite::Error::InvalidParameterName(format!("bad status {status}"))),
    }
    conn.execute("UPDATE reminders SET status = ?1 WHERE id = ?2", params![status, id])?;
    Ok(())
}

pub fn update_due(conn: &Connection, id: i64, due_at: &str) -> Result<()> {
    let now = chrono::Local::now().naive_local();
    let parsed = parse_due(due_at, now)
        .or_else(|| NaiveDateTime::parse_from_str(due_at, "%Y-%m-%dT%H:%M").ok())
        .or_else(|| NaiveDateTime::parse_from_str(due_at, "%Y-%m-%d %H:%M:%S").ok())
        .or_else(|| NaiveDateTime::parse_from_str(due_at, "%Y-%m-%d %H:%M").ok());
    let dt = match parsed {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        None => return Err(rusqlite::Error::InvalidParameterName("bad due format".into())),
    };
    conn.execute(
        "UPDATE reminders SET due_at = ?1, needs_time = 0 WHERE id = ?2",
        params![dt, id],
    )?;
    Ok(())
}

pub fn reminders_due_soon(conn: &Connection, now_iso: &str) -> Result<Vec<ReminderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at, status, needs_time, conversation_id
         FROM reminders
         WHERE status = 'pending' AND notified_at IS NULL AND due_at IS NOT NULL
           AND due_at <= ?1
         ORDER BY due_at ASC",
    )?;
    let rows = stmt.query_map(params![now_iso], map_row)?;
    rows.collect()
}

pub fn list_future_reminders(conn: &Connection, now_iso: &str) -> Result<Vec<ReminderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at, status, needs_time, conversation_id
         FROM reminders
         WHERE status = 'pending' AND notified_at IS NULL AND due_at IS NOT NULL
           AND due_at > ?1
         ORDER BY due_at ASC",
    )?;
    let rows = stmt.query_map(params![now_iso], map_row)?;
    rows.collect()
}

fn map_row(r: &rusqlite::Row) -> Result<ReminderRow> {
    Ok(ReminderRow {
        id: r.get(0)?,
        content: r.get(1)?,
        due_at: r.get(2)?,
        status: r.get(3)?,
        needs_time: r.get::<_, i64>(4)? != 0,
        conversation_id: r.get(5)?,
    })
}
