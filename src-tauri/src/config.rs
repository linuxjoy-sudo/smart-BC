use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub input_device: Option<usize>,
    #[serde(default)]
    pub voice_assistant_enabled: bool,
    #[serde(default = "default_wake_word")]
    pub wake_word: String,
    #[serde(default = "default_listen_window")]
    pub listen_window_secs: u32,
    #[serde(default)]
    pub wake_model: String,
    #[serde(default)]
    pub asr_model: String,
    #[serde(default = "default_reply_mode")]
    pub reply_mode: String,
    #[serde(default)]
    pub app_commands: HashMap<String, String>,
}

fn default_reply_mode() -> String {
    "notification".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            input_device: None,
            voice_assistant_enabled: false,
            wake_word: default_wake_word(),
            listen_window_secs: default_listen_window(),
            wake_model: String::new(),
            asr_model: String::new(),
            reply_mode: default_reply_mode(),
            app_commands: HashMap::new(),
        }
    }
}

fn default_wake_word() -> String { "小贝小贝".into() }
fn default_listen_window() -> u32 { 30 }

pub fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("config.json")
}

pub fn load_config(data_dir: &Path) -> Config {
    let path = config_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(data_dir: &Path, cfg: &Config) -> Result<(), String> {
    let path = config_path(data_dir);
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn load_api_key(data_dir: &Path) -> Option<String> {
    if let Ok(k) = std::env::var("DEEPSEEK_API_KEY") {
        if !k.is_empty() { return Some(k); }
    }
    let cfg = load_config(data_dir);
    if cfg.api_key.is_empty() { None } else { Some(cfg.api_key) }
}
