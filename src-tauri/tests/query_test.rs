use rusqlite::Connection;
use smart_bc::db;
use smart_bc::llm::provider::{LlmError, LlmProvider};
use std::cell::RefCell;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

struct MockProvider {
    system: RefCell<String>,
    reply: serde_json::Value,
}

impl LlmProvider for MockProvider {
    fn chat_json(&self, system: &str, _user: &str) -> Result<serde_json::Value, LlmError> {
        *self.system.borrow_mut() = system.to_string();
        Ok(self.reply.clone())
    }
}

#[test]
fn search_context_collects_hits_and_memories() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "周三和张伟开预算会", None).unwrap();
    let ext = smart_bc::memory::types::MemoryExtraction {
        people: vec![smart_bc::memory::types::PersonExtract { name: "张伟".into(), relation: "供应商".into(), note: String::new() }],
        reminders: vec![], preferences: vec![], episode: None,
    };
    db::memories::save_extraction(&conn, &ext, cid).unwrap();
    let ctx = smart_bc::query::search_context(&conn, "预算会", 10).unwrap();
    assert_eq!(ctx.hits.len(), 1);
    assert_eq!(ctx.people.len(), 1);
    assert_eq!(ctx.people[0].name, "张伟");
}

#[test]
fn answer_prompt_includes_question_and_evidence() {
    let p = smart_bc::llm::answer::build_answer_prompt(
        "上次和张伟聊了什么？",
        &[smart_bc::db::search::SearchHit {
            conversation_id: 1,
            transcript: "和张伟聊了预算".into(),
            snippet: "【和张伟聊了预算】".into(),
        }],
        &[],
        &[],
    );
    assert!(p.contains("上次和张伟聊了什么"));
    assert!(p.contains("【和张伟聊了预算】"));
}

#[test]
fn answer_system_prompt_contains_json() {
    let p = smart_bc::llm::answer::build_answer_prompt("问题", &[], &[], &[]);
    let provider = MockProvider { system: RefCell::new(String::new()), reply: serde_json::json!({"answer": "x"}) };
    let _ = smart_bc::llm::answer::answer_question(&provider, "问题", &[], &[], &[]);
    assert!(provider.system.borrow().to_lowercase().contains("json"), "DeepSeek json_object 要求 prompt 含 json");
    assert!(p.contains("问题"));
}

#[test]
fn answer_parses_answer_field() {
    let provider = MockProvider { system: RefCell::new(String::new()), reply: serde_json::json!({"answer": "明天早上八点去医院"}) };
    let ans = smart_bc::llm::answer::answer_question(&provider, "几点去医院", &[], &[], &[]).unwrap();
    assert_eq!(ans, "明天早上八点去医院");
}

#[test]
fn answer_falls_back_to_string() {
    let provider = MockProvider { system: RefCell::new(String::new()), reply: serde_json::json!("直接文本回答") };
    let ans = smart_bc::llm::answer::answer_question(&provider, "问题", &[], &[], &[]).unwrap();
    assert_eq!(ans, "直接文本回答");
}
