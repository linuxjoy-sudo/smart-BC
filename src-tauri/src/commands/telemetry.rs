use crate::app_state::AppState;
use crate::telemetry::{UsageStats, log_event, usage_stats};

#[tauri::command]
pub fn log_usage(state: tauri::State<'_, AppState>, event: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    log_event(&conn, &event).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_usage_stats(state: tauri::State<'_, AppState>) -> Result<UsageStats, String> {
    let conn = state.conn.lock().unwrap();
    usage_stats(&conn).map_err(|e| e.to_string())
}
