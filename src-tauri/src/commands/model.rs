use crate::app_state::AppState;
use crate::asr::whisper::Transcriber;

fn model_path(data_dir: &std::path::Path) -> std::path::PathBuf {
    crate::asr::model::model_path(data_dir)
}

#[tauri::command]
pub fn load_model(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let path = model_path(&state.data_dir);
    if !path.exists() {
        return Err(format!(
            "模型文件不存在：{}（请先下载 ggml-small.bin 到该路径）",
            path.display()
        ));
    }
    let transcriber = Transcriber::new(&path).map_err(|e| format!("模型加载失败: {e}"))?;
    let mut guard = state.transcriber.lock().unwrap();
    *guard = Some(transcriber);
    Ok(())
}

#[tauri::command]
pub fn download_model(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let path = model_path(&state.data_dir);
    if path.exists() {
        return Ok("模型已存在，无需下载".into());
    }
    crate::asr::model::download_model(crate::asr::model::MODEL_URL, &path)?;
    load_model(state)?;
    Ok(format!("模型下载并加载成功：{}", path.display()))
}
