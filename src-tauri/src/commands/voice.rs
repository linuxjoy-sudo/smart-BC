use crate::app_state::AppState;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static LISTENER_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn voice_assistant_enabled(data_dir: &Path) -> bool {
    crate::config::load_config(data_dir).voice_assistant_enabled
}

pub fn listener_running() -> bool {
    LISTENER_RUNNING.load(Ordering::Acquire)
}

pub fn try_start_listener(app: tauri::AppHandle, state: AppState) -> bool {
    if state.recorder.lock().unwrap().is_some() {
        return false;
    }
    if LISTENER_RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return false;
    }
    std::thread::spawn(move || {
        crate::voice::dialog::run_listener(app, state);
        LISTENER_RUNNING.store(false, Ordering::Release);
    });
    true
}

#[derive(Serialize)]
pub struct VoiceStatus {
    pub enabled: bool,
    pub state: String,
}

#[tauri::command]
pub fn set_voice_assistant(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<String, String> {
    if enabled {
        if state.recorder.lock().unwrap().is_some() {
            return Err("正在录音中，请先停止录音".into());
        }
        let mut cfg = crate::config::load_config(&state.data_dir);
        cfg.voice_assistant_enabled = true;
        crate::config::save_config(&state.data_dir, &cfg)?;
        let _ = try_start_listener(app, state.inner().clone());
        Ok("语音助手已开启".into())
    } else {
        let mut cfg = crate::config::load_config(&state.data_dir);
        cfg.voice_assistant_enabled = false;
        crate::config::save_config(&state.data_dir, &cfg)?;
        Ok("语音助手已关闭".into())
    }
}

#[tauri::command]
pub fn get_voice_status(state: tauri::State<'_, AppState>) -> Result<VoiceStatus, String> {
    Ok(VoiceStatus {
        enabled: voice_assistant_enabled(&state.data_dir),
        state: "idle".into(),
    })
}
