use smart_bc::commands::voice::voice_assistant_enabled;
use smart_bc::config;

#[test]
fn voice_assistant_disabled_by_default() {
    let dir = std::env::temp_dir().join("smartbc_voice_cmd_def");
    assert!(!voice_assistant_enabled(&dir));
}

#[test]
fn voice_assistant_enabled_after_config() {
    let dir = std::env::temp_dir().join("smartbc_voice_cmd_on");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config { voice_assistant_enabled: true, ..Default::default() };
    config::save_config(&dir, &cfg).unwrap();
    assert!(voice_assistant_enabled(&dir));
    std::fs::remove_file(config::config_path(&dir)).ok();
}
