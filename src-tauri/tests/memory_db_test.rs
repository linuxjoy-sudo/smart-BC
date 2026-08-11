use rusqlite::Connection;
use smart_bc::db;
use smart_bc::memory::types::{EpisodeExtract, MemoryExtraction, PersonExtract, PreferenceExtract, ReminderExtract};

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

fn sample_extraction() -> MemoryExtraction {
    MemoryExtraction {
        people: vec![PersonExtract { name: "张伟".into(), relation: "供应商".into(), note: String::new() }],
        reminders: vec![ReminderExtract { content: "周三交方案".into(), due: Some("2026-08-12".into()) }],
        preferences: vec![PreferenceExtract { topic: "饮食".into(), value: "不吃香菜".into() }],
        episode: Some(EpisodeExtract { summary: "讨论预算".into(), people: vec!["张伟".into()], place: Some("公司".into()) }),
    }
}

#[test]
fn save_and_list_all_memory_types() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "周三交方案给张伟，他不吃香菜", None).unwrap();
    db::memories::save_extraction(&conn, &sample_extraction(), cid).unwrap();
    let people = db::memories::list_people(&conn).unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].name, "张伟");
    assert_eq!(people[0].conversation_id, cid);
    let prefs = db::memories::list_preferences(&conn).unwrap();
    assert_eq!(prefs.len(), 1);
    assert_eq!(prefs[0].value, "不吃香菜");
    let eps = db::memories::list_episodes(&conn, 10).unwrap();
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].summary, "讨论预算");
}

#[test]
fn delete_memory_removes_row() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "认识李四", None).unwrap();
    db::memories::save_extraction(&conn, &sample_extraction(), cid).unwrap();
    let people = db::memories::list_people(&conn).unwrap();
    db::memories::delete_memory(&conn, "people", people[0].id).unwrap();
    assert!(db::memories::list_people(&conn).unwrap().is_empty());
}

#[test]
fn delete_memory_rejects_unknown_table() {
    let conn = mem_conn();
    assert!(db::memories::delete_memory(&conn, "users", 1).is_err());
}
