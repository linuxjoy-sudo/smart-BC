use crate::app_state::AppState;
use crate::config::{Config, save_config};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct ExportPayload {
    pub conversations: Vec<crate::db::conversations::ConversationRow>,
    pub people: Vec<crate::db::memories::PersonRow>,
    pub preferences: Vec<crate::db::memories::PreferenceRow>,
    pub episodes: Vec<crate::db::memories::EpisodeRow>,
    pub reminders: Vec<crate::db::reminders::ReminderRow>,
}

#[tauri::command]
pub fn save_api_key(state: tauri::State<'_, AppState>, key: String) -> Result<(), String> {
    let mut cfg = crate::config::load_config(&state.data_dir);
    cfg.api_key = key.clone();
    save_config(&state.data_dir, &cfg)?;
    let mut guard = state.llm.lock().unwrap();
    *guard = std::sync::Arc::new(crate::llm::client::DeepSeekClient::new(&key));
    Ok(())
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    Ok(crate::config::load_config(&state.data_dir))
}

#[tauri::command]
pub fn save_input_device(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    index: Option<usize>,
) -> Result<(), String> {
    let mut cfg = crate::config::load_config(&state.data_dir);
    cfg.input_device = index;
    crate::config::save_config(&state.data_dir, &cfg)?;
    if cfg.voice_assistant_enabled {
        crate::commands::voice::restart_listener(&app, state.inner())?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_reply_mode(state: tauri::State<'_, AppState>, mode: String) -> Result<(), String> {
    let mut cfg = crate::config::load_config(&state.data_dir);
    cfg.reply_mode = mode;
    save_config(&state.data_dir, &cfg)
}

#[tauri::command]
pub fn export_dir(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.data_dir.to_string_lossy().to_string())
}

pub fn delete_conversation_core(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM people WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM preferences WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM episodes WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM reminders WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM conversations_fts WHERE rowid = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM conversations WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

#[tauri::command]
pub fn delete_conversation(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    delete_conversation_core(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_all_data(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute_batch(
        "DELETE FROM conversations_fts;
         DELETE FROM people; DELETE FROM preferences; DELETE FROM episodes;
         DELETE FROM reminders; DELETE FROM conversations;",
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_all(state: tauri::State<'_, AppState>, dest: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let payload = ExportPayload {
        conversations: crate::commands::query::list_conversations_impl(&conn)?,
        people: crate::db::memories::list_people(&conn).map_err(|e| e.to_string())?,
        preferences: crate::db::memories::list_preferences(&conn).map_err(|e| e.to_string())?,
        episodes: crate::db::memories::list_episodes(&conn, 10000).map_err(|e| e.to_string())?,
        reminders: crate::db::reminders::list_reminders(&conn).map_err(|e| e.to_string())?,
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&dest, json).map_err(|e| e.to_string())?;
    Ok(dest)
}
