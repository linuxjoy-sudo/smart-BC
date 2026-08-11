use crate::db::memories::{PersonRow, PreferenceRow};
use crate::db::search::SearchHit;

#[derive(Debug, Default)]
pub struct QueryContext {
    pub hits: Vec<SearchHit>,
    pub people: Vec<PersonRow>,
    pub prefs: Vec<PreferenceRow>,
}

pub fn search_context(
    conn: &rusqlite::Connection,
    question: &str,
    limit: usize,
) -> Result<QueryContext, String> {
    let hits = crate::db::search::search_transcripts(conn, question, limit).map_err(|e| e.to_string())?;
    let people = crate::db::memories::list_people(conn).map_err(|e| e.to_string())?;
    let prefs = crate::db::memories::list_preferences(conn).map_err(|e| e.to_string())?;
    Ok(QueryContext { hits, people, prefs })
}
