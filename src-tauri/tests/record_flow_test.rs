use rusqlite::Connection;
use smart_bc::commands::record::store_transcript;
use smart_bc::db;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn store_transcript_persists_and_indexes() {
    let conn = mem_conn();
    let result = store_transcript(&conn, "周三交方案给张伟", None).unwrap();
    assert!(result.conversation_id > 0);
    let hits = db::search::search_transcripts(&conn, "方案", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].conversation_id, result.conversation_id);
}

#[test]
fn empty_transcript_rejected() {
    let conn = mem_conn();
    let err = store_transcript(&conn, "   ", None).unwrap_err();
    assert!(err.contains("空"));
}
