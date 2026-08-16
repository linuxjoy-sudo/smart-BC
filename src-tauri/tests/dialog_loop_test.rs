use rusqlite::Connection;
use smart_bc::app_state::AppState;
use smart_bc::config::{Config, save_config};
use smart_bc::db;
use smart_bc::llm::provider::{LlmError, LlmProvider};
use smart_bc::voice::dialog::run_loop;
use smart_bc::voice::feed::WavFeed;
use smart_bc::voice::reply::DialogSink;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct MockProvider;

impl LlmProvider for MockProvider {
    fn chat_json(&self, _system: &str, _user: &str) -> Result<serde_json::Value, LlmError> {
        Ok(serde_json::json!({"people": [], "reminders": [], "preferences": [], "episode": null}))
    }
}

struct MockSink {
    messages: Arc<Mutex<Vec<String>>>,
}

impl DialogSink for MockSink {
    fn deliver(&self, _mode: &str, message: String) {
        self.messages.lock().unwrap().push(message);
    }
    fn notify_error(&self, _body: &str) {}
    fn tts_playing(&self) -> bool {
        false
    }
}

fn model_dir() -> Option<PathBuf> {
    std::env::var("SMARTBC_MODEL_DIR").ok().map(PathBuf::from)
}

fn test_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_dir = dir.path().to_path_buf();
    let cfg = Config {
        voice_assistant_enabled: true,
        wake_word: "小贝小贝".into(),
        listen_window_secs: 30,
        ..Config::default()
    };
    save_config(&data_dir, &cfg).unwrap();
    let conn = Arc::new(Mutex::new({
        let c = Connection::open_in_memory().unwrap();
        db::schema::migrate(&c).unwrap();
        c
    }));
    let transcriber = model_dir()
        .map(|d| {
            let p = d.join("ggml-base.bin");
            if p.exists() {
                smart_bc::asr::whisper::Transcriber::new(&p).ok()
            } else {
                None
            }
        })
        .unwrap_or(None);
    let llm = Arc::new(Mutex::new(
        Arc::new(MockProvider) as Arc<dyn LlmProvider + Send + Sync>
    ));
    let state = AppState {
        conn,
        recorder: Arc::new(Mutex::new(None)),
        transcriber: Arc::new(Mutex::new(transcriber)),
        wake_transcriber: Arc::new(Mutex::new(None)),
        data_dir,
        llm,
    };
    (dir, state)
}

fn run_wav(state: &AppState, sink: &MockSink, fixture: &str, tail_ms: u32) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(fixture);
    let mut feed = WavFeed::from_wav(&path, tail_ms).expect("load fixture");
    let app = tauri::test::mock_builder().build(tauri::generate_context!()).unwrap();
    let handle = app.handle().clone();
    run_loop(&handle, state, &mut feed, sink);
}

#[test]
fn wake_fixture_triggers_active() {
    let (_dir, state) = test_state();
    if state.transcriber.lock().unwrap().is_none() {
        eprintln!("SKIP: SMARTBC_MODEL_DIR/ggml-base.bin 未就绪");
        return;
    }
    let sink = MockSink { messages: Arc::new(Mutex::new(Vec::new())) };
    run_wav(&state, &sink, "wake.wav", 2000);
    let msgs = sink.messages.lock().unwrap().clone();
    assert!(
        msgs.iter().any(|m| m.contains("在呢，请说")),
        "唤醒应触发'在呢，请说'，实际播报={msgs:?}"
    );
}

#[test]
fn non_wake_audio_stays_idle() {
    let (_dir, state) = test_state();
    if state.transcriber.lock().unwrap().is_none() {
        eprintln!("SKIP: SMARTBC_MODEL_DIR/ggml-base.bin 未就绪");
        return;
    }
    let sink = MockSink { messages: Arc::new(Mutex::new(Vec::new())) };
    run_wav(&state, &sink, "reminder.wav", 2000);
    let msgs = sink.messages.lock().unwrap().clone();
    assert!(
        !msgs.iter().any(|m| m.contains("在呢，请说")),
        "非唤醒词不应触发唤醒，实际播报={msgs:?}"
    );
}
