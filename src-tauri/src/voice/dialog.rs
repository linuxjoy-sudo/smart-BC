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
    use crate::voice::log::{log_error, log_line};
    use crate::voice::vad::{rms, EnergyVad};
    use crate::voice::wake::contains_wake_word;
    use std::time::{Duration, Instant};
    use tauri_plugin_notification::NotificationExt;

    let cfg = load_config(&state.data_dir);
    let wake_word = cfg.wake_word.clone();
    let window = Duration::from_secs(cfg.listen_window_secs.max(1) as u64);
    log_line(&state.data_dir, &format!("run_listener 启动: wake_word={wake_word:?} window={}s", window.as_secs()));
    let listener = match Listener::start(None, 5) {
        Ok(l) => l,
        Err(e) => {
            log_error(&state.data_dir, &format!("Listener 启动失败: {e}"));
            let _ = app.notification().builder().title("SmartBC 语音助手").body(format!("启动失败：{e}")).show();
            return;
        }
    };
    log_line(&state.data_dir, &format!("Listener 就绪: sample_rate={} 缓冲=5s", listener.sample_rate));
    let transcriber = state.transcriber.lock().unwrap().clone();
    let transcriber = match transcriber {
        Some(t) => t,
        None => {
            log_error(&state.data_dir, "模型未加载，语音助手不可用");
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
    let mut was_speaking = false;
    let mut last_vad_log = Instant::now();

    loop {
        if !voice_assistant_enabled(&state.data_dir) {
            log_line(&state.data_dir, "已禁用 → run_listener 退出");
            let _ = listener.stop();
            return;
        }
        let snap = listener.buffer.lock().unwrap().snapshot();
        if snap.is_empty() { std::thread::sleep(Duration::from_millis(100)); continue; }
        let speaking = vad.feed(&snap);
        buf.extend_from_slice(&snap);
        listener.buffer.lock().unwrap().clear();
        let level = rms(&snap);

        if last_vad_log.elapsed().as_secs_f64() >= 1.0 {
            last_vad_log = Instant::now();
            log_line(
                &state.data_dir,
                &format!("VAD: state={state_machine:?} speaking={speaking} was_speaking={was_speaking} level={level:.4} buf_ms={}",
                    buf.len() * 1000 / sr),
            );
        }
        if speaking && !was_speaking {
            log_line(&state.data_dir, &format!("语音突发开始: level={level:.4}"));
        }

        match state_machine {
            DialogState::Idle => {
                if !speaking && was_speaking {
                    let burst_rms = rms(&buf);
                    log_line(&state.data_dir, &format!("语音突发结束: burst_rms={burst_rms:.4} buf_ms={}", buf.len() * 1000 / sr));
                    if burst_rms > 0.01 {
                        let chunk: Vec<f32> = buf.split_off(buf.len().saturating_sub(sr * 3));
                        log_line(&state.data_dir, &format!("唤醒转写: chunk_ms={}", chunk.len() * 1000 / sr));
                        match transcriber.transcribe_samples(listener.sample_rate, &chunk) {
                            Ok(t) => {
                                let matched = contains_wake_word(&t, &wake_word);
                                log_line(&state.data_dir, &format!("唤醒转写文本: {t:?} matched={matched}"));
                                if matched {
                                    state_machine = transition(state_machine, DialogEvent::WakeWordHit);
                                    active_start = Instant::now();
                                    log_line(&state.data_dir, "唤醒命中 → Active，发送\"在呢，请说\"通知");
                                    let _ = app.notification().builder().title("SmartBC").body("在呢，请说").show();
                                }
                            }
                            Err(e) => log_error(&state.data_dir, &format!("唤醒转写失败: {e}")),
                        }
                    }
                    buf.clear();
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
                            let transcribed = transcriber.transcribe_samples(listener.sample_rate, &sentence);
                            log_line(&state.data_dir, &format!("断句转写: sentence_ms={}", sentence.len() * 1000 / sr));
                            let result = match transcribed {
                                Ok(text) => {
                                    log_line(&state.data_dir, &format!("问句: {text:?}"));
                                    let conn = state.conn.lock().unwrap();
                                    let llm = state.llm.lock().unwrap().clone();
                                    answer_question_core(&conn, llm.as_ref(), &text)
                                }
                                Err(e) => Err(e),
                            };
                            match result {
                                Ok(ans) => {
                                    log_line(&state.data_dir, &format!("回答: {ans:?}"));
                                    let _ = app.notification().builder().title("SmartBC").body(ans).show();
                                    state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                }
                                Err(e) => {
                                    log_error(&state.data_dir, &format!("问答失败: {e}"));
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
                    log_line(&state.data_dir, "聆听窗口超时 → Idle");
                    state_machine = transition(state_machine, DialogEvent::WindowTimeout);
                    buf.clear();
                }
            }
            DialogState::Processing => {}
        }
        was_speaking = speaking;
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
