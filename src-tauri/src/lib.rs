pub mod app_state;
pub mod asr;
pub mod audio;
pub mod commands;
pub mod config;
pub mod db;
pub mod llm;
pub mod memory;
pub mod query;
pub mod scheduler;
pub mod telemetry;
pub mod timeparse;
pub mod voice;

use app_state::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[tauri::command]
fn ping() -> String {
    "pong".into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("smartbc");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = data_dir.join("smartbc.db");
    let conn = db::open(&db_path).expect("open db");
    let api_key = config::load_api_key(&data_dir).unwrap_or_default();
    let llm = Arc::new(Mutex::new(
        Arc::new(llm::client::DeepSeekClient::new(&api_key))
            as Arc<dyn llm::provider::LlmProvider + Send + Sync>
    ));
    let model_path = asr::model::model_path(&data_dir);
    let transcriber = if model_path.exists() {
        match asr::whisper::Transcriber::new(&model_path) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("模型加载失败（可到设置页重新加载）: {e}");
                None
            }
        }
    } else {
        eprintln!("模型文件不存在: {}", model_path.display());
        None
    };
    let state = AppState {
        conn: Arc::new(Mutex::new(conn)),
        recorder: Arc::new(Mutex::new(None)),
        transcriber: Arc::new(Mutex::new(transcriber)),
        data_dir,
        llm,
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .setup(|app| {
            if let Some(state) = app.try_state::<AppState>() {
                let conn_arc = state.conn.clone();
                let handle = app.handle().clone();
                scheduler::spawn(conn_arc, handle);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::record::start_recording,
            commands::record::stop_recording,
            commands::record::list_audio_devices,
            commands::record::get_transcription_status,
            commands::query::query_memories,
            commands::query::list_conversations,
            commands::query::list_reminders_cmd,
            commands::query::list_people_cmd,
            commands::query::list_preferences_cmd,
            commands::query::complete_reminder,
            commands::settings::save_api_key,
            commands::settings::get_config,
            commands::settings::save_input_device,
            commands::settings::delete_conversation,
            commands::settings::clear_all_data,
            commands::settings::export_all,
            commands::settings::export_dir,
            commands::model::load_model,
            commands::model::download_model,
            commands::telemetry::log_usage,
            commands::telemetry::get_usage_stats,
            commands::voice::set_voice_assistant,
            commands::voice::get_voice_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
