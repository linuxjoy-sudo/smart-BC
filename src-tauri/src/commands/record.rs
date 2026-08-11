use crate::app_state::AppState;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RecordResult {
    pub conversation_id: i64,
    pub transcript: String,
}

/// 纯入库函数（可单测）：原文先入库，保证数据不丢。
pub fn store_transcript(
    conn: &Connection,
    transcript: &str,
    audio_path: Option<&str>,
) -> Result<RecordResult, String> {
    let t = transcript.trim();
    if t.is_empty() {
        return Err("转写结果为空，请重试".into());
    }
    let id = crate::db::conversations::insert_conversation(conn, t, audio_path)
        .map_err(|e| format!("入库失败: {e}"))?;
    Ok(RecordResult { conversation_id: id, transcript: t.to_string() })
}

#[tauri::command]
pub fn start_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.recorder.lock().unwrap();
    if guard.is_some() {
        return Err("正在录音中".into());
    }
    let recorder = crate::audio::recorder::Recorder::new(None).map_err(|e| e.to_string())?;
    recorder.start().map_err(|e| e.to_string())?;
    *guard = Some(recorder);
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: tauri::State<'_, AppState>) -> Result<RecordResult, String> {
    let mut guard = state.recorder.lock().unwrap();
    let recorder = guard.take().ok_or("没有正在进行的录音")?;
    let dir = state.data_dir.join("recordings");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let wav_path = dir.join(format!("rec_{stamp}.wav"));
    recorder.stop_and_save(&wav_path).map_err(|e| e.to_string())?;

    let trans_guard = state.transcriber.lock().unwrap();
    let transcriber = trans_guard
        .as_ref()
        .ok_or("模型未加载，请先在设置中下载模型")?;
    let transcript = transcriber.transcribe(&wav_path)?;
    let conn = state.conn.lock().unwrap();
    store_transcript(&conn, &transcript, Some(wav_path.to_str().unwrap()))
}

#[tauri::command]
pub fn get_transcription_status(state: tauri::State<'_, AppState>) -> bool {
    state.transcriber.lock().unwrap().is_some()
}
