use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub conversation_id: i64,
    pub transcript: String,
    pub snippet: String,
}

pub fn search_transcripts(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let trimmed = query.trim();
    if trimmed.len() < 2 {
        return Ok(Vec::new());
    }
    // trigram 分词要求查询串 >= 3 字符；中文 2 字词按片段用 LIKE 兜底
    let sql = if trimmed.chars().count() >= 3 {
        r#"SELECT rowid, transcript,
                  snippet(conversations_fts, 0, '【', '】', '…', 12) AS snip
           FROM conversations_fts
           WHERE transcript MATCH ?1
           ORDER BY rank
           LIMIT ?2"#
    } else {
        r#"SELECT id, transcript, transcript AS snip
           FROM conversations
           WHERE transcript LIKE '%' || ?1 || '%'
           ORDER BY id DESC
           LIMIT ?2"#
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![trimmed, limit as i64], |r| {
        Ok(SearchHit {
            conversation_id: r.get(0)?,
            transcript: r.get(1)?,
            snippet: r.get(2)?,
        })
    })?;
    rows.collect()
}
