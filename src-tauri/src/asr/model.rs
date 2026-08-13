use std::path::{Path, PathBuf};

pub const MODEL_FILENAME: &str = "ggml-small.bin";
pub const WAKE_MODEL_FILENAME: &str = "ggml-base.bin";
pub const MODEL_URL: &str =
    "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";
pub const WAKE_MODEL_URL: &str =
    "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-base.bin";

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
