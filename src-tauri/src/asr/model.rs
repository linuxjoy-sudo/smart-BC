use std::path::{Path, PathBuf};

pub const MODEL_FILENAME: &str = "ggml-small.bin";
pub const WAKE_MODEL_FILENAME: &str = "ggml-base.bin";
pub const MODEL_URL: &str =
    "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";
pub const WAKE_MODEL_URL: &str =
    "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";
pub const MEDIUM_MODEL_FILENAME: &str = "ggml-medium.bin";
pub const MEDIUM_MODEL_URL: &str =
    "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin";

/// 按配置选择 ASR（内容转写）模型文件名：small（默认，快）/ medium（准，慢）
pub fn asr_model_filename(asr_model: &str) -> &'static str {
    match asr_model {
        "medium" => MEDIUM_MODEL_FILENAME,
        _ => MODEL_FILENAME,
    }
}

pub fn model_path(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join(MODEL_FILENAME)
}

pub fn wake_model_path(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join(WAKE_MODEL_FILENAME)
}

pub fn download_model(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let resp = reqwest::blocking::get(url).map_err(|e| format!("http: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download failed: {}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| format!("read body: {e}"))?;
    std::fs::write(dest, bytes).map_err(|e| e.to_string())?;
    Ok(())
}
