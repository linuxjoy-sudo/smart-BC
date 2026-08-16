use std::collections::HashMap;
use std::path::Path;

/// 内置应用映射表（名称 → 命令/路径）
pub fn builtin_apps() -> &'static [(&'static str, &'static str)] {
    &[
        ("计算器", "calc.exe"),
        ("记事本", "notepad.exe"),
        ("画图", "mspaint.exe"),
        ("控制面板", "control"),
        ("我的电脑", "explorer"),
        ("命令提示符", "cmd"),
        ("回收站", "explorer shell:RecycleBinFolder"),
        ("音乐", "wmplayer.exe"),
        ("媒体播放器", "wmplayer.exe"),
    ]
}

/// 合并映射：内置 + config 覆盖/新增
pub fn app_map(data_dir: &Path) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = builtin_apps()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let cfg = crate::config::load_config(data_dir);
    for (k, v) in &cfg.app_commands {
        map.insert(k.clone(), v.clone());
    }
    map
}

/// 查询应用映射（精确名，去"打开/启动/开启"等前缀后）
pub fn resolve_app<'a>(map: &'a HashMap<String, String>, name: &str) -> Option<&'a String> {
    let n = name.trim().to_string();
    if let Some(v) = map.get(&n) {
        return Some(v);
    }
    // 容错：名称可能带后缀（如"计算器吧"）——精确优先，fallback 包含匹配
    map.iter()
        .find(|(k, _)| n.contains(k.as_str()))
        .map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("smartbc_apps_test");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn builtin_has_music_and_common() {
        let map: HashMap<String, String> = builtin_apps()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert_eq!(map.get("计算器").map(|s| s.as_str()), Some("calc.exe"));
        assert_eq!(map.get("音乐").map(|s| s.as_str()), Some("wmplayer.exe"));
        assert!(map.contains_key("记事本"));
    }

    #[test]
    fn config_overrides_builtin() {
        let dir = test_dir();
        let mut cfg = crate::config::Config::default();
        cfg.app_commands.insert("音乐".into(), "D:/music/custom.exe".into());
        crate::config::save_config(&dir, &cfg).unwrap();
        let map = app_map(&dir);
        assert_eq!(map.get("音乐").map(|s| s.as_str()), Some("D:/music/custom.exe"));
    }

    #[test]
    fn resolve_exact_and_fuzzy() {
        let mut map = HashMap::new();
        map.insert("计算器".to_string(), "calc.exe".to_string());
        assert_eq!(resolve_app(&map, "计算器").map(|s| s.as_str()), Some("calc.exe"));
        assert_eq!(resolve_app(&map, "打开计算器").map(|s| s.as_str()), Some("calc.exe"));
        assert_eq!(resolve_app(&map, "不存在的应用"), None);
    }
}
