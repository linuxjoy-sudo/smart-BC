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

pub fn restart_listener<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &AppState) -> Result<(), String> {
    use std::time::Duration;
    let mut cfg = crate::config::load_config(&state.data_dir);
    if !cfg.voice_assistant_enabled {
        return Ok(());
    }
    cfg.voice_assistant_enabled = false;
    crate::config::save_config(&state.data_dir, &cfg)?;
    for _ in 0..40 {
        if !listener_running() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let st = state.clone();
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        let mut cfg2 = crate::config::load_config(&st.data_dir);
        cfg2.voice_assistant_enabled = true;
        let _ = crate::config::save_config(&st.data_dir, &cfg2);
        try_start_listener(app2, st);
    });
    Ok(())
}

pub fn try_start_listener<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: AppState) -> bool {
    if state.recorder.lock().unwrap().is_some() {
        crate::voice::log::log_error(&state.data_dir, "try_start_listener: 正在录音中，拒绝启动监听");
        return false;
    }
    if LISTENER_RUNNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        crate::voice::log::log_line(&state.data_dir, "try_start_listener: 监听已在运行（CAS 失败），跳过");
        return false;
    }
    crate::voice::log::log_line(&state.data_dir, "try_start_listener: 启动监听线程");
    let data_dir = state.data_dir.clone();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::voice::dialog::run_listener(app, state);
        }));
        if let Err(payload) = result {
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic payload".to_string()
            };
            crate::voice::log::log_error(&data_dir, &format!("run_listener 线程 PANIC: {msg}"));
        }
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
        let started = try_start_listener(app, state.inner().clone());
        crate::voice::log::log_line(&state.data_dir, &format!("set_voice_assistant(true): 已保存 config，try_start_listener={started}"));
        Ok("语音助手已开启".into())
    } else {
        let mut cfg = crate::config::load_config(&state.data_dir);
        cfg.voice_assistant_enabled = false;
        crate::config::save_config(&state.data_dir, &cfg)?;
        crate::voice::log::log_line(&state.data_dir, "set_voice_assistant(false): 已保存 config（监听线程将于轮询时退出）");
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
