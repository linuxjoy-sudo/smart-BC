use crate::asr::whisper::Transcriber;
use crate::audio::recorder::Recorder;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub recorder: Arc<Mutex<Option<Recorder>>>,
    pub transcriber: Arc<Mutex<Option<Transcriber>>>,
    pub data_dir: PathBuf,
    pub llm: Arc<Mutex<Arc<dyn crate::llm::provider::LlmProvider + Send + Sync>>>,
}
