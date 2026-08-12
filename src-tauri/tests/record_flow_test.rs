use rusqlite::Connection;
use smart_bc::commands::record::{resolve_device_index, store_transcript};
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

#[test]
fn resolve_device_requested_persists() {
    let dir = std::env::temp_dir().join("smartbc_rec_dev_test");
    std::fs::create_dir_all(&dir).unwrap();
    let idx = resolve_device_index(&dir, Some(3)).unwrap();
    assert_eq!(idx, Some(3));
    let cfg = smart_bc::config::load_config(&dir);
    assert_eq!(cfg.input_device, Some(3));
    std::fs::remove_file(smart_bc::config::config_path(&dir)).ok();
}

#[test]
fn resolve_device_falls_back_to_saved() {
    let dir = std::env::temp_dir().join("smartbc_rec_dev_test2");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = smart_bc::config::Config { input_device: Some(5), ..Default::default() };
    smart_bc::config::save_config(&dir, &cfg).unwrap();
    let idx = resolve_device_index(&dir, None).unwrap();
    assert_eq!(idx, Some(5));
    std::fs::remove_file(smart_bc::config::config_path(&dir)).ok();
}

#[test]
fn resolve_device_none_when_not_saved() {
    let dir = std::env::temp_dir().join("smartbc_rec_dev_test3");
    std::fs::create_dir_all(&dir).unwrap();
    let idx = resolve_device_index(&dir, None).unwrap();
    assert_eq!(idx, None);
}
