use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ConversationRow {
    pub id: i64,
    pub created_at: String,
    pub transcript: String,
    pub summary: Option<String>,
    pub audio_path: Option<String>,
}

pub fn insert_conversation(
    conn: &Connection,
    transcript: &str,
    audio_path: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO conversations (transcript, audio_path) VALUES (?1, ?2)",
        params![transcript, audio_path],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO conversations_fts (rowid, transcript) VALUES (?1, ?2)",
        params![id, transcript],
    )?;
    Ok(id)
}

pub fn update_summary(conn: &Connection, id: i64, summary: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET summary = ?1 WHERE id = ?2",
        params![summary, id],
    )?;
    Ok(())
}

pub fn get_conversation(conn: &Connection, id: i64) -> Result<Option<ConversationRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, transcript, summary, audio_path FROM conversations WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |r| {
        Ok(ConversationRow {
            id: r.get(0)?,
            created_at: r.get(1)?,
            transcript: r.get(2)?,
            summary: r.get(3)?,
            audio_path: r.get(4)?,
        })
    })?;
    rows.next().transpose()
}
