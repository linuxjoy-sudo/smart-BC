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
use tauri::menu::{Menu, MenuItem};
use tauri::Manager;

#[tauri::command]
fn ping() -> String {
    "pong".into()
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开设置界面", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle_voice", "语音助手：开", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &quit])?;
    let mut builder = tauri::tray::TrayIconBuilder::with_id("smartbc-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                    let _ = w.eval("window.location.hash = '#settings'");
                }
            }
            "toggle_voice" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let enabled = !commands::voice::voice_assistant_enabled(&state.data_dir);
                    let _ = commands::voice::set_voice_assistant(
                        app.clone(),
                        app.state::<AppState>(),
                        enabled,
                    );
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn cfg_uses_wake_model(data_dir: &std::path::Path) -> bool {
    let m = config::load_config(data_dir).wake_model;
    m == "base" || m == "tiny"
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
    let wake_transcriber = if cfg_uses_wake_model(&data_dir) {
        let wake_path = asr::model::wake_model_path(&data_dir);
        if wake_path.exists() {
            match asr::whisper::Transcriber::new(&wake_path) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("唤醒模型加载失败，回退主模型: {e}");
                    None
                }
            }
        } else {
            eprintln!("唤醒模型不存在（{}），回退主模型", wake_path.display());
            None
        }
    } else {
        None
    };
    let state = AppState {
        conn: Arc::new(Mutex::new(conn)),
        recorder: Arc::new(Mutex::new(None)),
        transcriber: Arc::new(Mutex::new(transcriber)),
        wake_transcriber: Arc::new(Mutex::new(wake_transcriber)),
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
                scheduler::spawn(conn_arc, handle, state.data_dir.clone());
                if crate::commands::voice::voice_assistant_enabled(&state.data_dir) {
                    crate::commands::voice::try_start_listener(app.handle().clone(), state.inner().clone());
                }
                let _ = build_tray(app.handle());
                // 首次引导：无 API key 或模型缺失 → 显示设置窗口
                let cfg = crate::config::load_config(&state.data_dir);
                let model_ok = crate::asr::model::model_path(&state.data_dir).exists();
                if cfg.api_key.is_empty() || !model_ok {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                    }
                }
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
            commands::query::update_reminder_due,
            commands::settings::save_api_key,
            commands::settings::get_config,
            commands::settings::save_input_device,
            commands::settings::save_reply_mode,
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
