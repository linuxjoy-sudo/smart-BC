use rusqlite::Connection;
use smart_bc::config;
use smart_bc::db;

#[allow(dead_code)]
fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn config_roundtrip() {
    let dir = std::env::temp_dir().join("smartbc_cfg_test");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config { api_key: "sk-test-123".into() };
    config::save_config(&dir, &cfg).unwrap();
    let loaded = config::load_config(&dir);
    assert_eq!(loaded.api_key, "sk-test-123");
    std::fs::remove_file(config::config_path(&dir)).ok();
}

#[test]
fn env_var_takes_priority() {
    unsafe { std::env::set_var("DEEPSEEK_API_KEY", "sk-env"); }
    let dir = std::env::temp_dir().join("smartbc_cfg_test2");
    let key = config::load_api_key(&dir);
    assert_eq!(key.as_deref(), Some("sk-env"));
    unsafe { std::env::remove_var("DEEPSEEK_API_KEY"); }
}
