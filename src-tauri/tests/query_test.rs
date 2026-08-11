use rusqlite::Connection;
use smart_bc::db;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
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
