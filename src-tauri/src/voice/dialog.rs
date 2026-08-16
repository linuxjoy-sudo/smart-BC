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

pub fn run_listener<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: crate::app_state::AppState) {
    use crate::config::load_config;
    use crate::voice::listener::Listener;
    use crate::voice::log::{log_error, log_line};
    use crate::voice::vad::rms;
    use std::time::Duration;
    use tauri_plugin_notification::NotificationExt;

    let cfg = load_config(&state.data_dir);
    let input_device = cfg.input_device;
    let window = Duration::from_secs(cfg.listen_window_secs.max(1) as u64);
    log_line(&state.data_dir, &format!(
        "run_listener 启动: wake_word={:?} window={}s input_device={input_device:?}",
        cfg.wake_word,
        window.as_secs()
    ));
    let listener = match Listener::start(input_device, 5) {
        Ok(l) => l,
        Err(e) => {
            log_error(&state.data_dir, &format!("Listener 启动失败: {e}"));
            if let Err(ne) = app.notification().builder().title("SmartBC 语音助手").body(format!("启动失败：{e}")).show() {
                log_error(&state.data_dir, &format!("通知发送失败: {ne}"));
            }
            return;
        }
    };
    let device_name = crate::audio::recorder::list_input_devices()
        .ok()
        .and_then(|devs| {
            devs.iter()
                .find(|d| Some(d.index) == input_device)
                .map(|d| d.name.clone())
        })
        .unwrap_or_else(|| "未知".into());
    log_line(&state.data_dir, &format!(
        "Listener 就绪: sample_rate={} 缓冲=5s 设备[{input_device:?}]={device_name}",
        listener.sample_rate
    ));
    std::thread::sleep(Duration::from_millis(1500));
    let init_level = rms(&listener.buffer.lock().unwrap().snapshot());
    log_line(&state.data_dir, &format!("启动拾音电平: {init_level:.4}"));
    if init_level < 0.002 {
        log_error(&state.data_dir, "麦克风未拾音（检查 Windows 隐私-麦克风权限，或设备未工作）");
        if let Err(ne) = app.notification()
            .builder()
            .title("SmartBC 语音助手")
            .body("麦克风未拾音：请检查 Windows 隐私-麦克风权限，或确认录音设备工作正常")
            .show()
        {
            log_error(&state.data_dir, &format!("通知发送失败: {ne}"));
        }
    }
    let mut feed = crate::voice::feed::CpalFeed { listener };
    let sink = crate::voice::reply::AppSink { app: &app, data_dir: &state.data_dir };
    run_loop(&app, &state, &mut feed, &sink);
}

/// 核心对话循环：从 feed 取音频、驱动状态机（唤醒→聆听→断句→转写→处理）。
/// 独立于真实麦克风（CpalFeed）与真实播报（AppSink），可注入 WavFeed/MockSink 测试。
pub fn run_loop<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &crate::app_state::AppState,
    feed: &mut dyn crate::voice::feed::AudioFeed,
    sink: &dyn crate::voice::reply::DialogSink,
) {
    use crate::commands::voice::voice_assistant_enabled;
    use crate::config::load_config;
    use crate::voice::log::{log_error, log_line};
    use crate::voice::vad::{rms, EnergyVad};
    use crate::voice::wake::contains_wake_word;
    use std::time::{Duration, Instant};

    let cfg = load_config(&state.data_dir);
    let reply_mode = cfg.reply_mode.clone();
    let wake_word = cfg.wake_word.clone();
    let window = Duration::from_secs(cfg.listen_window_secs.max(1) as u64);

    let transcriber = state.transcriber.lock().unwrap().clone();
    let transcriber = match transcriber {
        Some(t) => t,
        None => {
            log_error(&state.data_dir, "模型未加载，语音助手不可用");
            sink.notify_error("模型未加载，语音助手不可用");
            return;
        }
    };
    let wake_transcriber = state
        .wake_transcriber
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| transcriber.clone());
    log_line(&state.data_dir, "唤醒模型: base（若未加载则回退 small）");
    let mut wake_state = match wake_transcriber.new_state() {
        Ok(s) => s,
        Err(e) => {
            log_error(&state.data_dir, &format!("唤醒状态创建失败: {e}"));
            return;
        }
    };
    let mut sentence_state = match transcriber.new_state() {
        Ok(s) => s,
        Err(e) => {
            log_error(&state.data_dir, &format!("问答状态创建失败: {e}"));
            return;
        }
    };
    let mut vad = EnergyVad::new(0.02, (feed.sample_rate() / 100).max(1) as usize);
    let sr = feed.sample_rate() as usize;
    let mut buf: Vec<f32> = Vec::new();
    let mut state_machine = DialogState::Idle;
    let mut active_start = Instant::now();
    let mut silence_since = Instant::now();
    let mut was_speaking = false;
    let mut last_vad_log = Instant::now();
    let mut last_wake_check = Instant::now() - Duration::from_secs(3);
    let mut pending_time: Option<(i64, String, u32)> = None;

    loop {
        if !voice_assistant_enabled(&state.data_dir) {
            log_line(&state.data_dir, "已禁用 → run_listener 退出");
            
            return;
        }
        let snap = match feed.next_samples(sr / 20) {
            Some(s) => s,
            None => {
                log_line(&state.data_dir, "音频流结束 → run_loop 退出");
                return;
            }
        };
        if snap.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        if sink.tts_playing() {
            buf.clear();
            silence_since = Instant::now();
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let speaking = vad.feed(&snap);
        buf.extend_from_slice(&snap);
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

        if state_machine == DialogState::Idle
            && crate::scheduler::PENDING_REMINDER_ID.load(std::sync::atomic::Ordering::SeqCst) > 0
        {
            state_machine = DialogState::Active;
            active_start = Instant::now();
            silence_since = Instant::now();
            log_line(&state.data_dir, "提醒触发，进入聆听响应窗口（可说\"完成\"或\"延后X分钟\"）");
        }

        match state_machine {
            DialogState::Idle => {
                if !speaking && was_speaking {
                    let burst_rms = rms(&buf);
                    let buf_ms = buf.len() * 1000 / sr;
                    log_line(&state.data_dir, &format!("语音突发结束: burst_rms={burst_rms:.4} buf_ms={}", buf_ms));
                    let throttled = last_wake_check.elapsed().as_secs_f64() < 2.0;
                    if burst_rms > 0.01 && buf_ms >= 300 && !throttled {
                        last_wake_check = Instant::now();
                        let chunk = trim_to_voice(&buf, sr);
                        if chunk.is_empty() {
                            log_line(&state.data_dir, "语音段为空，跳过唤醒转写");
                        } else {
                            log_line(&state.data_dir, &format!("唤醒转写: chunk_ms={} 开始", chunk.len() * 1000 / sr));
                            let t0 = Instant::now();
                            match wake_transcriber.transcribe_with_state(&mut wake_state, feed.sample_rate(), &chunk) {
                                Ok(t) => {
                                    let matched = contains_wake_word(&t, &wake_word);
                                    log_line(&state.data_dir, &format!("唤醒转写文本: {t:?} matched={matched} (耗时 {:?})", t0.elapsed()));
                                    if matched {
                                        state_machine = transition(state_machine, DialogEvent::WakeWordHit);
                                        active_start = Instant::now();
                                        silence_since = Instant::now();
                                        log_line(&state.data_dir, "唤醒命中 → Active，发送\"在呢，请说\"回复");
                                        sink.deliver(&reply_mode, "在呢，请说".into());
                                    }
                                }
                                Err(e) => log_error(&state.data_dir, &format!("唤醒转写失败: {e} (耗时 {:?})", t0.elapsed())),
                            }
                        }
                    } else if throttled {
                        log_line(&state.data_dir, "唤醒转写节流跳过（距上次 <2s）");
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
                            let t0 = Instant::now();
                            // 先用 base 快速检测唤醒词，命中则直接重置窗口（避免 small 慢转写）
                            let wake_text = wake_transcriber.transcribe_with_state(&mut wake_state, feed.sample_rate(), &sentence);
                            let re_wake = match &wake_text {
                                Ok(t) => {
                                    contains_wake_word(t, &wake_word)
                                        && t.chars().count() <= wake_word.chars().count() * 2
                                }
                                Err(_) => false,
                            };
                            if re_wake {
                                log_line(&state.data_dir, &format!("重复唤醒词（base 快速检测，耗时 {:?}）→ 重置聆听窗口", t0.elapsed()));
                                state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                sink.deliver(&reply_mode, "在呢，请说".into());
                            } else {
                                let transcribed = transcriber.transcribe_with_state(&mut sentence_state, feed.sample_rate(), &sentence);
                                log_line(&state.data_dir, &format!("断句转写完成 (耗时 {:?})", t0.elapsed()));
                                let re_wake_small = match &transcribed {
                                    Ok(t) => {
                                        contains_wake_word(t, &wake_word)
                                            && t.chars().count() <= wake_word.chars().count() * 2
                                    }
                                    Err(_) => false,
                                };
                                if re_wake_small {
                                    log_line(&state.data_dir, "重复唤醒词（small 二次检测命中）→ 重置聆听窗口");
                                    state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                    sink.deliver(&reply_mode, "在呢，请说".into());
                                } else if let Some(rid) = {
                                    let v = crate::scheduler::PENDING_REMINDER_ID.load(std::sync::atomic::Ordering::SeqCst);
                                    (v > 0).then_some(v)
                                } {
                                    let response = match &transcribed {
                                        Ok(text) => handle_reminder_response(text),
                                        Err(_) => ReminderResponse::Unknown,
                                    };
                                    match response {
                                        ReminderResponse::Done => {
                                            let conn = state.conn.lock().unwrap();
                                            let _ = crate::db::reminders::set_status(&conn, rid, "done");
                                            crate::scheduler::PENDING_REMINDER_ID.store(0, std::sync::atomic::Ordering::SeqCst);
                                            log_line(&state.data_dir, &format!("提醒 {rid} 语音完成"));
                                            sink.deliver(&reply_mode, "好的，已帮你完成".into());
                                        }
                                        ReminderResponse::Deferred(dt) => {
                                            let due = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                                            let conn = state.conn.lock().unwrap();
                                            match crate::db::reminders::update_due(&conn, rid, &due) {
                                                Ok(_) => {
                                                    crate::scheduler::PENDING_REMINDER_ID.store(0, std::sync::atomic::Ordering::SeqCst);
                                                    log_line(&state.data_dir, &format!("提醒 {rid} 延后到 {due}"));
                                                    sink.deliver(&reply_mode, format!("好的，延后到{}", friendly_time(dt)));
                                                }
                                                Err(e) => log_error(&state.data_dir, &format!("延后失败: {e}")),
                                            }
                                        }
                                        ReminderResponse::Unknown => {
                                            log_line(&state.data_dir, "提醒响应未识别，提示指令");
                                            sink.deliver(&reply_mode, "可以说\"完成\"或\"延后5分钟\"".into());
                                        }
                                    }
                                    state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                } else if let Some((id, content, attempts)) = pending_time.take() {
                                    let parsed = match &transcribed {
                                        Ok(text) => {
                                            let now = chrono::Local::now().naive_local();
                                            crate::timeparse::parse_due(text, now)
                                        }
                                        Err(_) => None,
                                    };
                                    match parsed {
                                        Some(dt) => {
                                            let due = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                                            let conn = state.conn.lock().unwrap();
                                            match crate::db::reminders::update_due(&conn, id, &due) {
                                                Ok(_) => {
                                                    log_line(&state.data_dir, &format!("语音补时间成功: {content} => {due}"));
                                                    sink.deliver(&reply_mode, format!("好的，{}提醒你{}", friendly_time(dt), content));
                                                }
                                                Err(e) => log_error(&state.data_dir, &format!("补时间失败: {e}")),
                                            }
                                        }
                                        None => {
                                            if attempts < 2 {
                                                let tx = match &transcribed {
                                                    Ok(t) => format!("{t:?}"),
                                                    Err(e) => format!("转写失败: {e}"),
                                                };
                                                log_line(&state.data_dir, &format!("补时间未解析（转写={tx}），追问 {content}"));
                                                pending_time = Some((id, content, attempts + 1));
                                                sink.deliver(&reply_mode, "没听清时间，请再说一次，比如'下午三点'".into());
                                            } else {
                                                log_line(&state.data_dir, &format!("补时间多次失败，放弃 {content}"));
                                                sink.deliver(&reply_mode, format!("暂时没设好时间，之后可以重新提醒我{}", content));
                                            }
                                        }
                                    }
                                    state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                } else {
                                    let system_cmd = match &transcribed {
                                        Ok(text) => crate::voice::commands::parse_system_command(text),
                                        Err(_) => crate::voice::commands::SystemCommand::None,
                                    };
                                    if !matches!(system_cmd, crate::voice::commands::SystemCommand::None) {
                                        let conn = state.conn.lock().unwrap();
                                        let msg = crate::voice::commands::execute_system_command(app, state, &conn, system_cmd);
                                        drop(conn);
                                        log_line(&state.data_dir, &format!("系统指令: {msg:?}"));
                                        sink.deliver(&reply_mode, msg);
                                        state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                    } else {
                                    let result = match transcribed {
                                        Ok(text) => {
                                            log_line(&state.data_dir, &format!("转写: {text:?}"));
                                            let conn = state.conn.lock().unwrap();
                                            let llm = state.llm.lock().unwrap().clone();
                                            process_transcript(&conn, llm.as_ref(), &text)
                                        }
                                        Err(e) => Err(e),
                                    };
                                match result {
                                    Ok(outcome) => {
                                        let (label, message) = match outcome {
                                            TranscriptOutcome::Recorded(msg) => ("已记录", msg),
                                            TranscriptOutcome::Answered(ans) => ("回答", ans),
                                            TranscriptOutcome::NeedsTime(id, content) => {
                                                log_line(&state.data_dir, &format!("无时间提醒，语音追问: {content}"));
                                                pending_time = Some((id, content.clone(), 0));
                                                let clean = crate::memory::extract::clean_reminder_content(&content);
                                                ("追问时间", format!("好的，什么时候提醒你{}？", clean))
                                            }
                                        };
                                        log_line(&state.data_dir, &format!("{label}: {message:?}"));
                                        sink.deliver(&reply_mode, message);
                                        state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                    }
                                        Err(e) => {
                                            log_error(&state.data_dir, &format!("处理失败: {e}"));
                                            sink.notify_error(&format!("查询失败：{e}"));
                                            state_machine = transition(state_machine, DialogEvent::ProcessedErr);
                                        }
                                    }
                                    }
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
                    if crate::scheduler::PENDING_REMINDER_ID.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                        crate::scheduler::PENDING_REMINDER_ID.store(0, std::sync::atomic::Ordering::SeqCst);
                    }
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

pub enum TranscriptOutcome {
    Recorded(String),
    Answered(String),
    NeedsTime(i64, String),
}

enum ReminderResponse {
    Done,
    Deferred(chrono::NaiveDateTime),
    Unknown,
}

fn handle_reminder_response(text: &str) -> ReminderResponse {
    let done_keys = ["完成", "好了", "行了", "知道了", "可以了", "收到", "搞定"];
    let defer_keys = ["延后", "再等", "晚点", "过一会", "过会儿", "稍后", "推迟"];
    if done_keys.iter().any(|k| text.contains(k)) {
        return ReminderResponse::Done;
    }
    if defer_keys.iter().any(|k| text.contains(k)) {
        let now = chrono::Local::now().naive_local();
        if let Some(dt) = crate::timeparse::parse_due(text, now) {
            return ReminderResponse::Deferred(dt);
        }
    }
    ReminderResponse::Unknown
}

pub fn process_transcript(
    conn: &rusqlite::Connection,
    llm: &dyn crate::llm::provider::LlmProvider,
    text: &str,
) -> Result<TranscriptOutcome, String> {
    use crate::commands::record::store_transcript;
    use crate::memory::extract::extract_from_transcript;

    let result = store_transcript(conn, text, None)?;
    match extract_from_transcript(llm, text) {
        Ok(ext) => {
            let has_content = !ext.reminders.is_empty()
                || !ext.people.is_empty()
                || !ext.preferences.is_empty()
                || ext.episode.is_some();
            if !ext.reminders.is_empty() {
                let now = chrono::Local::now().naive_local();
                let ids = crate::db::reminders::save_reminders(conn, &ext.reminders, result.conversation_id, now)
                    .map_err(|e| format!("提醒入库失败: {e}"))?;
                for id in ids {
                    if let Ok(Some(r)) = crate::db::reminders::get_reminder(conn, id) {
                        if r.needs_time {
                            return Ok(TranscriptOutcome::NeedsTime(r.id, r.content));
                        }
                    }
                }
            }
            if !ext.people.is_empty() || !ext.preferences.is_empty() || ext.episode.is_some() {
                crate::db::memories::save_extraction(conn, &ext, result.conversation_id)
                    .map_err(|e| format!("记忆入库失败: {e}"))?;
            }
            if has_content {
                let msg = build_record_message(&ext);
                let _ = crate::db::conversations::update_summary(conn, result.conversation_id, &msg);
                Ok(TranscriptOutcome::Recorded(msg))
            } else {
                let ans = crate::commands::query::answer_question_core(conn, llm, text)?;
                let _ = crate::db::conversations::update_summary(conn, result.conversation_id, &ans);
                Ok(TranscriptOutcome::Answered(ans))
            }
        }
        Err(_e) => {
            let ans = crate::commands::query::answer_question_core(conn, llm, text)?;
            let _ = crate::db::conversations::update_summary(conn, result.conversation_id, &ans);
            Ok(TranscriptOutcome::Answered(ans))
        }
    }
}

fn trim_to_voice(buf: &[f32], sr: usize) -> Vec<f32> {
    const THRESHOLD: f32 = 0.02;
    let frame_len = (sr / 100).max(1);
    let frame_count = buf.len() / frame_len;
    let mut first: Option<usize> = None;
    let mut last: Option<usize> = None;
    for i in 0..frame_count {
        let frame = &buf[i * frame_len..(i + 1) * frame_len];
        if crate::voice::vad::rms(frame) > THRESHOLD {
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
        }
    }
    let (f, l) = match (first, last) {
        (Some(f), Some(l)) => (f, l),
        _ => return Vec::new(),
    };
    let pad = frame_len * 15;
    let start = f * frame_len - pad.min(f * frame_len);
    let end = ((l + 1) * frame_len + pad).min(buf.len());
    buf[start..end].to_vec()
}

fn build_record_message(ext: &crate::memory::types::MemoryExtraction) -> String {
    let mut parts = Vec::new();
    for r in &ext.reminders {
        parts.push(format!("提醒：{}", r.content));
    }
    for p in &ext.people {
        parts.push(format!("人脉：{}（{}）", p.name, p.relation));
    }
    for pr in &ext.preferences {
        parts.push(format!("偏好：{}：{}", pr.topic, pr.value));
    }
    if let Some(ep) = &ext.episode {
        parts.push(format!("事件：{}", ep.summary));
    }
    format!("已记录：{}", parts.join("；"))
}

fn friendly_time(dt: chrono::NaiveDateTime) -> String {
    let now = chrono::Local::now().naive_local();
    let day = if dt.date() == now.date() {
        "今天".to_string()
    } else if dt.date() == now.date().succ_opt().unwrap_or(now.date()) {
        "明天".to_string()
    } else {
        dt.format("%m月%d日").to_string()
    };
    format!("{day}{}", dt.format("%H:%M"))
}
