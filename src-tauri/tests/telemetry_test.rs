use rusqlite::Connection;
use smart_bc::db;
use smart_bc::telemetry::{log_event, usage_stats};

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn log_and_count_events() {
    let conn = mem_conn();
    log_event(&conn, "recording_done").unwrap();
    log_event(&conn, "recording_done").unwrap();
    log_event(&conn, "query_asked").unwrap();
    let stats = usage_stats(&conn).unwrap();
    assert_eq!(stats.recordings, 2);
    assert_eq!(stats.queries, 1);
}

#[test]
fn rejects_unknown_events() {
    let conn = mem_conn();
    assert!(log_event(&conn, "DROP TABLE conversations").is_err());
}
