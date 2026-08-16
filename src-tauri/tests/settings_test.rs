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
    let cfg = config::Config { api_key: "sk-test-123".into(), input_device: Some(1), ..Default::default() };
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
    let cfg = config::Config { api_key: "sk-old".into(), input_device: Some(2), ..Default::default() };
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
fn voice_assistant_config_defaults() {
    let dir = std::env::temp_dir().join("smartbc_voice_cfg_default");
    let cfg = config::load_config(&dir);
    assert!(!cfg.voice_assistant_enabled);
    assert_eq!(cfg.wake_word, "小贝小贝");
    assert_eq!(cfg.listen_window_secs, 30);
    assert_eq!(cfg.wake_model, "");
}

#[test]
fn voice_assistant_config_roundtrip() {
    let dir = std::env::temp_dir().join("smartbc_voice_cfg_rt");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config {
        api_key: String::new(),
        input_device: None,
        voice_assistant_enabled: true,
        wake_word: "你好助手".into(),
        listen_window_secs: 45,
        wake_model: "base".into(),
        asr_model: "small".into(),
        reply_mode: "notification".into(),
    };
    config::save_config(&dir, &cfg).unwrap();
    let loaded = config::load_config(&dir);
    assert!(loaded.voice_assistant_enabled);
    assert_eq!(loaded.wake_word, "你好助手");
    assert_eq!(loaded.listen_window_secs, 45);
    assert_eq!(loaded.wake_model, "base");
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
