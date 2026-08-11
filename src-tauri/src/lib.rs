pub mod app_state;
pub mod asr;
pub mod audio;
pub mod commands;
pub mod db;

use app_state::AppState;
use std::sync::{Arc, Mutex};

#[tauri::command]
fn ping() -> String {
    "pong".into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("smartbc");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = data_dir.join("smartbc.db");
    let conn = db::open(&db_path).expect("open db");
    let state = AppState {
        conn: Arc::new(Mutex::new(conn)),
        recorder: Arc::new(Mutex::new(None)),
        transcriber: Arc::new(Mutex::new(None)),
        data_dir,
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::record::start_recording,
            commands::record::stop_recording,
            commands::record::get_transcription_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
