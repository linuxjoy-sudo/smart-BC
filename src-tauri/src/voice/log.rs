use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const LOG_FILE: &str = "voice-assistant.log";

pub fn log_line(data_dir: &Path, msg: &str) {
    let path = data_dir.join(LOG_FILE);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{now}] {msg}");
    }
}

pub fn log_error(data_dir: &Path, msg: &str) {
    log_line(data_dir, &format!("ERROR: {msg}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_line_writes_to_file() {
        let dir = std::env::temp_dir().join("smartbc_voice_log_test");
        std::fs::create_dir_all(&dir).unwrap();
        log_line(&dir, "hello 测试");
        log_error(&dir, "boom");
        let content = std::fs::read_to_string(dir.join(LOG_FILE)).unwrap();
        assert!(content.contains("hello 测试"));
        assert!(content.contains("ERROR: boom"));
        std::fs::remove_file(dir.join(LOG_FILE)).ok();
    }
}
