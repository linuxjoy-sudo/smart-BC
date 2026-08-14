#[cfg(windows)]
pub fn speak_async(data_dir: &std::path::Path, text: String) {
    use crate::voice::log::{log_error, log_line};
    use std::time::{Duration, Instant};
    let dir = data_dir.to_path_buf();
    std::thread::spawn(move || {
        match tts::Tts::new(tts::Backends::WinRt) {
            Ok(mut tts) => {
                if let Ok(Some(v)) = tts.voice() {
                    log_line(&dir, &format!("TTS 当前语音: {}", v.name()));
                }
                match tts.speak(text, true) {
                    Ok(_) => log_line(&dir, "TTS 播放已提交"),
                    Err(e) => {
                        log_error(&dir, &format!("TTS 播放失败: {e}"));
                        return;
                    }
                }
                // 保持 Tts（MediaPlayer）存活直到播放完成或超时，
                // 否则线程退出 drop Tts 会中断异步播放
                let deadline = Instant::now() + Duration::from_secs(10);
                while Instant::now() < deadline {
                    if !tts.is_speaking().unwrap_or(false) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
            Err(e) => log_error(&dir, &format!("TTS 初始化失败: {e}")),
        }
    });
}

#[cfg(not(windows))]
pub fn speak_async(_data_dir: &std::path::Path, _text: String) {
    eprintln!("TTS 播报仅在 Windows 可用");
}
