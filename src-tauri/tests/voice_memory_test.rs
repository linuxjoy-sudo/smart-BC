use rusqlite::Connection;
use smart_bc::db;
use smart_bc::llm::provider::{LlmError, LlmProvider};
use smart_bc::voice::dialog::{TranscriptOutcome, process_transcript};
use std::cell::RefCell;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

struct MockProvider {
    reply: RefCell<serde_json::Value>,
    calls: RefCell<usize>,
}

impl LlmProvider for MockProvider {
    fn chat_json(&self, _system: &str, _user: &str) -> Result<serde_json::Value, LlmError> {
        *self.calls.borrow_mut() += 1;
        Ok(self.reply.borrow().clone())
    }
}

fn reminder_reply(content: &str) -> serde_json::Value {
    serde_json::json!({"people": [], "reminders": [{"content": content, "due": null}], "preferences": [], "episode": null})
}

fn empty_reply() -> serde_json::Value {
    serde_json::json!({"people": [], "reminders": [], "preferences": [], "episode": null})
}

fn answer_reply() -> serde_json::Value {
    serde_json::json!({"answer": "明天上午10点"})
}

#[test]
fn records_reminder_instruction() {
    let conn = mem_conn();
    let p = MockProvider { reply: RefCell::new(reminder_reply("明天中午12点提醒我去吃饭")), calls: RefCell::new(0) };
    let out = process_transcript(&conn, &p, "明天中午12点提醒我去吃饭").unwrap();
    match out {
        TranscriptOutcome::Recorded(msg) => assert!(msg.contains("提醒")),
        _ => panic!("应返回 Recorded"),
    }
    let reminders = db::reminders::list_reminders(&conn).unwrap();
    assert!(!reminders.is_empty(), "提醒应入库");
    assert!(reminders[0].content.contains("提醒我去吃饭"));
}

#[test]
fn records_people_statement() {
    let conn = mem_conn();
    let reply = serde_json::json!({"people": [{"name": "张伟", "relation": "供应商", "note": ""}], "reminders": [], "preferences": [], "episode": null});
    let p = MockProvider { reply: RefCell::new(reply), calls: RefCell::new(0) };
    let out = process_transcript(&conn, &p, "张伟是供应商").unwrap();
    match out {
        TranscriptOutcome::Recorded(msg) => assert!(msg.contains("人脉")),
        _ => panic!("应返回 Recorded"),
    }
    let people = db::memories::list_people(&conn).unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].name, "张伟");
}

#[test]
fn answers_question_when_no_extraction() {
    let conn = mem_conn();
    let p = MockProvider { reply: RefCell::new(empty_reply()), calls: RefCell::new(0) };
    let out = process_transcript(&conn, &p, "明天几点开会").unwrap();
    // 第一次调用是抽取（返回 empty），第二次是问答
    p.reply.replace(answer_reply());
    match out {
        TranscriptOutcome::Answered(_) => {}
        _ => panic!("空抽取应回退问答"),
    }
}

#[test]
fn answers_question_when_extraction_fails() {
    let conn = mem_conn();
    // 抽取返回非法 JSON → extract_from_transcript 报错 → 回退问答
    let p = MockProvider { reply: RefCell::new(serde_json::json!("not an object")), calls: RefCell::new(0) };
    let out = process_transcript(&conn, &p, "明天几点开会").unwrap();
    match out {
        TranscriptOutcome::Answered(_) => {}
        _ => panic!("抽取失败应回退问答"),
    }
    assert!(*p.calls.borrow() >= 2, "抽取失败后应再调用一次问答");
}
