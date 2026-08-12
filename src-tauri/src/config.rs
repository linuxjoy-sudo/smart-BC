use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub input_device: Option<usize>,
}

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
