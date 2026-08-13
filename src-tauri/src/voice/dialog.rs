#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogState {
    Idle,
    Active,
    Processing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogEvent {
    VoiceStart,
    WakeWordHit,
    SentenceEnd,
    WindowTimeout,
    ProcessedOk,
    ProcessedErr,
    MicUnavailable,
}

pub fn window_expired(active_elapsed: std::time::Duration, window: std::time::Duration) -> bool {
    active_elapsed >= window
}

pub fn silence_exceeded(silence_secs: f64, limit_secs: f64) -> bool {
    silence_secs >= limit_secs
}

pub fn run_listener(app: tauri::AppHandle, state: crate::app_state::AppState) {
    use crate::commands::query::answer_question_core;
    use crate::commands::voice::voice_assistant_enabled;
    use crate::config::load_config;
    use crate::voice::listener::Listener;
    use crate::voice::vad::{rms, EnergyVad};
    use crate::voice::wake::contains_wake_word;
    use std::time::{Duration, Instant};
    use tauri_plugin_notification::NotificationExt;

    let cfg = load_config(&state.data_dir);
    let wake_word = cfg.wake_word.clone();
    let window = Duration::from_secs(cfg.listen_window_secs.max(1) as u64);
    let listener = match Listener::start(None, 5) {
        Ok(l) => l,
        Err(e) => {
            let _ = app.notification().builder().title("SmartBC 语音助手").body(format!("启动失败：{e}")).show();
            return;
        }
    };
    let transcriber = state.transcriber.lock().unwrap().clone();
    let transcriber = match transcriber {
        Some(t) => t,
        None => {
            let _ = app.notification().builder().title("SmartBC 语音助手").body("模型未加载，语音助手不可用").show();
            return;
        }
    };
    let mut vad = EnergyVad::new(0.02, (listener.sample_rate / 100).max(1) as usize);
    let sr = listener.sample_rate as usize;
    let mut buf: Vec<f32> = Vec::new();
    let mut state_machine = DialogState::Idle;
    let mut active_start = Instant::now();
    let mut silence_since = Instant::now();

    loop {
        if !voice_assistant_enabled(&state.data_dir) {
            let _ = listener.stop();
            return;
        }
        let snap = listener.buffer.lock().unwrap().snapshot();
        if snap.is_empty() { std::thread::sleep(Duration::from_millis(100)); continue; }
        let speaking = vad.feed(&snap);
        buf.extend_from_slice(&snap);
        listener.buffer.lock().unwrap().clear();

        match state_machine {
            DialogState::Idle => {
                if speaking && buf.len() > sr * 3 {
                    let chunk: Vec<f32> = buf.split_off(buf.len().saturating_sub(sr * 3));
                    match transcriber.transcribe_samples(listener.sample_rate, &chunk) {
                        Ok(t) if contains_wake_word(&t, &wake_word) => {
                            state_machine = transition(state_machine, DialogEvent::WakeWordHit);
                            active_start = Instant::now();
                            let _ = app.notification().builder().title("SmartBC").body("在呢，请说").show();
                            buf.clear();
                        }
                        _ => { buf.clear(); }
                    }
                } else if buf.len() > sr * 5 {
                    buf.clear();
                }
            }
            DialogState::Active => {
                if !speaking {
                    if silence_since.elapsed().as_secs_f64() >= 1.5 {
                        let sentence: Vec<f32> = std::mem::take(&mut buf);
                        if !sentence.is_empty() && rms(&sentence) > 0.01 {
                            state_machine = transition(state_machine, DialogEvent::SentenceEnd);
                            let conn = state.conn.lock().unwrap();
                            let llm = state.llm.lock().unwrap().clone();
                            let result = transcriber
                                .transcribe_samples(listener.sample_rate, &sentence)
                                .and_then(|text| answer_question_core(&conn, llm.as_ref(), &text));
                            match result {
                                Ok(ans) => {
                                    let _ = app.notification().builder().title("SmartBC").body(ans).show();
                                    state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                }
                                Err(e) => {
                                    let _ = app.notification().builder().title("SmartBC").body(format!("查询失败：{e}")).show();
                                    state_machine = transition(state_machine, DialogEvent::ProcessedErr);
                                }
                            }
                            active_start = Instant::now();
                            silence_since = Instant::now();
                        }
                    }
                } else {
                    silence_since = Instant::now();
                }
                if window_expired(active_start.elapsed(), window) {
                    state_machine = transition(state_machine, DialogEvent::WindowTimeout);
                    buf.clear();
                }
            }
            DialogState::Processing => {}
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn transition(state: DialogState, event: DialogEvent) -> DialogState {
    match (state, event) {
        (DialogState::Idle, DialogEvent::WakeWordHit) => DialogState::Active,
        (DialogState::Active, DialogEvent::SentenceEnd) => DialogState::Processing,
        (DialogState::Processing, DialogEvent::ProcessedOk) => DialogState::Active,
        (DialogState::Processing, DialogEvent::ProcessedErr) => DialogState::Active,
        (DialogState::Active, DialogEvent::WindowTimeout) => DialogState::Idle,
        (_, DialogEvent::MicUnavailable) => DialogState::Idle,
        _ => state,
    }
}
