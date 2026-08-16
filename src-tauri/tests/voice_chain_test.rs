use rusqlite::Connection;
use smart_bc::asr::whisper::Transcriber;
use smart_bc::db;
use smart_bc::llm::provider::{LlmError, LlmProvider};
use smart_bc::voice::dialog::{TranscriptOutcome, process_transcript};
use smart_bc::voice::wake::contains_wake_word;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::OnceLock;

fn model_dir() -> Option<PathBuf> {
    std::env::var("SMARTBC_MODEL_DIR").ok().map(PathBuf::from)
}

fn base_model() -> Option<&'static Transcriber> {
    static T: OnceLock<Option<Transcriber>> = OnceLock::new();
    T.get_or_init(|| {
        let dir = model_dir()?;
        let path = dir.join("ggml-base.bin");
        if path.exists() {
            Transcriber::new(&path).ok()
        } else {
            None
        }
    })
    .as_ref()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

struct MockProvider {
    reply: RefCell<serde_json::Value>,
}

impl LlmProvider for MockProvider {
    fn chat_json(&self, _system: &str, _user: &str) -> Result<serde_json::Value, LlmError> {
        Ok(self.reply.borrow().clone())
    }
}

#[test]
fn wake_fixture_triggers_wake_word() {
    let Some(t) = base_model() else {
        eprintln!("SKIP: SMARTBC_MODEL_DIR/ggml-base.bin 未就绪");
        return;
    };
    let text = t.transcribe(&fixture("wake.wav")).expect("转写 wake.wav");
    assert!(contains_wake_word(&text, "小贝小贝"), "wake.wav 应命中唤醒词, 转写={text:?}");
}

#[test]
fn reminder_fixture_transcribes_content() {
    let Some(t) = base_model() else {
        eprintln!("SKIP: SMARTBC_MODEL_DIR/ggml-base.bin 未就绪");
        return;
    };
    let text = t.transcribe(&fixture("reminder.wav")).expect("转写 reminder.wav");
    assert!(text.contains("提醒"), "reminder.wav 应含'提醒', 转写={text:?}");
    assert!(text.contains("水"), "reminder.wav 应含'水', 转写={text:?}");
}

#[test]
fn reminder_chain_stores_reminder() {
    // 纯逻辑层：转写文本 → process_transcript(MockProvider) → 提醒入库
    let conn = mem_conn();
    let reply = serde_json::json!({
        "people": [], "preferences": [], "episode": null,
        "reminders": [{"content": "喝水", "due": "3分钟后"}]
    });
    let p = MockProvider { reply: RefCell::new(reply) };
    let out = process_transcript(&conn, &p, "3分钟后提醒我喝水").unwrap();
    match out {
        TranscriptOutcome::Recorded(msg) => assert!(msg.contains("提醒")),
        _ => panic!("带时间提醒应返回 Recorded"),
    }
    let reminders = db::reminders::list_reminders(&conn).unwrap();
    assert!(!reminders.is_empty(), "提醒应入库");
    assert!(reminders[0].content.contains("水"));
    assert!(reminders[0].due_at.is_some(), "due 应解析入库");
}

#[test]
fn reminder_chain_asks_time_when_missing() {
    // 无时间提醒 → NeedsTime 追问
    let conn = mem_conn();
    let reply = serde_json::json!({
        "people": [], "preferences": [], "episode": null,
        "reminders": [{"content": "喝水", "due": null}]
    });
    let p = MockProvider { reply: RefCell::new(reply) };
    let out = process_transcript(&conn, &p, "提醒我喝水").unwrap();
    match out {
        TranscriptOutcome::NeedsTime(_id, content) => assert_eq!(content, "喝水"),
        _ => panic!("无时间提醒应返回 NeedsTime"),
    }
}
