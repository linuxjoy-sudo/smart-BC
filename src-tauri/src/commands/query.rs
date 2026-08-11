use crate::app_state::AppState;
use crate::db::conversations::ConversationRow;
use crate::db::memories::{PersonRow, PreferenceRow};
use crate::db::reminders::ReminderRow;
use crate::llm::answer;

#[tauri::command]
pub fn query_memories(state: tauri::State<'_, AppState>, question: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let ctx = crate::query::search_context(&conn, &question, 8)?;
    let llm = state.llm.clone();
    let answer = answer::answer_question(llm.as_ref(), &question, &ctx.hits, &ctx.people, &ctx.prefs)
        .map_err(|e| e.to_string())?;
    Ok(answer)
}

#[tauri::command]
pub fn list_conversations(state: tauri::State<'_, AppState>) -> Result<Vec<ConversationRow>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, created_at, transcript, audio_path FROM conversations ORDER BY id DESC LIMIT 100",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(ConversationRow {
        id: r.get(0)?, created_at: r.get(1)?, transcript: r.get(2)?, audio_path: r.get(3)?,
    })).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<ConversationRow>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_reminders_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<ReminderRow>, String> {
    let conn = state.conn.lock().unwrap();
    crate::db::reminders::list_reminders(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_people_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<PersonRow>, String> {
    let conn = state.conn.lock().unwrap();
    crate::db::memories::list_people(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_preferences_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<PreferenceRow>, String> {
    let conn = state.conn.lock().unwrap();
    crate::db::memories::list_preferences(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_reminder(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    crate::db::reminders::set_status(&conn, id, "done").map_err(|e| e.to_string())
}
