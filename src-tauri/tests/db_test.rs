use rusqlite::Connection;
use smart_bc::db;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn insert_and_retrieve_conversation() {
    let conn = mem_conn();
    let id = db::conversations::insert_conversation(&conn, "周三给妈妈订蛋糕", None).unwrap();
    let row = db::conversations::get_conversation(&conn, id).unwrap().unwrap();
    assert_eq!(row.transcript, "周三给妈妈订蛋糕");
    assert!(row.id > 0);
}

#[test]
fn fts_finds_chinese_substring() {
    let conn = mem_conn();
    db::conversations::insert_conversation(&conn, "下周三和张伟开预算会", None).unwrap();
    let hits = db::search::search_transcripts(&conn, "预算", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains("预算"));
}

#[test]
fn fts_returns_empty_for_missing() {
    let conn = mem_conn();
    let hits = db::search::search_transcripts(&conn, "不存在的词xyz", 10).unwrap();
    assert!(hits.is_empty());
}
