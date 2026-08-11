use rusqlite::Connection;
use smart_bc::commands::settings::delete_conversation_core;
use smart_bc::db;
use smart_bc::memory::types::{MemoryExtraction, PersonExtract, ReminderExtract};

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

fn fixed_now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 8, 10).unwrap().and_hms_opt(9, 0, 0).unwrap()
}

#[test]
fn delete_conversation_cascades() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "认识李四并约定周五见面", None).unwrap();
    let ext = MemoryExtraction {
        people: vec![PersonExtract { name: "李四".into(), relation: "朋友".into(), note: String::new() }],
        reminders: vec![ReminderExtract { content: "周五见面".into(), due: Some("周五".into()) }],
        preferences: vec![], episode: None,
    };
    db::memories::save_extraction(&conn, &ext, cid).unwrap();
    db::reminders::save_reminders(&conn, &ext.reminders, cid, fixed_now()).unwrap();
    delete_conversation_core(&conn, cid).unwrap();
    assert!(db::conversations::get_conversation(&conn, cid).unwrap().is_none());
    assert!(db::memories::list_people(&conn).unwrap().is_empty());
    assert!(db::reminders::list_reminders(&conn).unwrap().is_empty());
}
