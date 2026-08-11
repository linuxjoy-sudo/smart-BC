use crate::memory::types::MemoryExtraction;
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PersonRow {
    pub id: i64,
    pub name: String,
    pub relation: String,
    pub note: String,
    pub conversation_id: i64,
}

#[derive(Debug, Serialize)]
pub struct PreferenceRow {
    pub id: i64,
    pub topic: String,
    pub value: String,
    pub conversation_id: i64,
}

#[derive(Debug, Serialize)]
pub struct EpisodeRow {
    pub id: i64,
    pub summary: String,
    pub place: String,
    pub conversation_id: i64,
}

pub fn save_extraction(conn: &Connection, ext: &MemoryExtraction, conversation_id: i64) -> Result<()> {
    for p in &ext.people {
        if p.name.trim().is_empty() { continue; }
        conn.execute(
            "INSERT INTO people (name, relation, note, conversation_id) VALUES (?1,?2,?3,?4)",
            params![p.name.trim(), p.relation.trim(), p.note.trim(), conversation_id],
        )?;
    }
    for pr in &ext.preferences {
        if pr.topic.trim().is_empty() { continue; }
        conn.execute(
            "INSERT INTO preferences (topic, value, conversation_id) VALUES (?1,?2,?3)",
            params![pr.topic.trim(), pr.value.trim(), conversation_id],
        )?;
    }
    if let Some(e) = &ext.episode {
        if !e.summary.trim().is_empty() {
            conn.execute(
                "INSERT INTO episodes (summary, place, conversation_id) VALUES (?1,?2,?3)",
                params![e.summary.trim(), e.place.clone().unwrap_or_default().trim(), conversation_id],
            )?;
        }
    }
    Ok(())
}

pub fn list_people(conn: &Connection) -> Result<Vec<PersonRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, relation, note, conversation_id FROM people ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok(PersonRow {
        id: r.get(0)?, name: r.get(1)?, relation: r.get(2)?,
        note: r.get(3)?, conversation_id: r.get(4)?,
    }))?;
    rows.collect()
}

pub fn list_preferences(conn: &Connection) -> Result<Vec<PreferenceRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, topic, value, conversation_id FROM preferences ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok(PreferenceRow {
        id: r.get(0)?, topic: r.get(1)?, value: r.get(2)?, conversation_id: r.get(3)?,
    }))?;
    rows.collect()
}

pub fn list_episodes(conn: &Connection, limit: usize) -> Result<Vec<EpisodeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, summary, place, conversation_id FROM episodes ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |r| Ok(EpisodeRow {
        id: r.get(0)?, summary: r.get(1)?, place: r.get(2)?, conversation_id: r.get(3)?,
    }))?;
    rows.collect()
}

const ALLOWED_TABLES: [&str; 3] = ["people", "preferences", "episodes"];

pub fn delete_memory(conn: &Connection, table: &str, id: i64) -> Result<()> {
    if !ALLOWED_TABLES.contains(&table) {
        return Err(rusqlite::Error::InvalidParameterName(format!("table {table} not allowed")));
    }
    conn.execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id])?;
    Ok(())
}
