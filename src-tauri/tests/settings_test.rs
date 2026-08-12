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
    let cfg = config::Config { api_key: "sk-test-123".into(), input_device: Some(1) };
    config::save_config(&dir, &cfg).unwrap();
    let loaded = config::load_config(&dir);
    assert_eq!(loaded.api_key, "sk-test-123");
    assert_eq!(loaded.input_device, Some(1));
    std::fs::remove_file(config::config_path(&dir)).ok();
}

#[test]
fn input_device_defaults_to_none() {
    let dir = std::env::temp_dir().join("smartbc_cfg_test3");
    let cfg = config::load_config(&dir);
    assert_eq!(cfg.input_device, None);
}

#[test]
fn api_key_update_preserves_input_device() {
    let dir = std::env::temp_dir().join("smartbc_cfg_test4");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config { api_key: "sk-old".into(), input_device: Some(2) };
    config::save_config(&dir, &cfg).unwrap();
    let mut updated = config::load_config(&dir);
    updated.api_key = "sk-new".into();
    config::save_config(&dir, &updated).unwrap();
    let loaded = config::load_config(&dir);
    assert_eq!(loaded.api_key, "sk-new");
    assert_eq!(loaded.input_device, Some(2));
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
