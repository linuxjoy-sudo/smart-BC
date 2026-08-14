#[cfg(windows)]
pub fn speak_async(data_dir: &std::path::Path, text: String) {
    use crate::voice::log::{log_error, log_line};
    let dir = data_dir.to_path_buf();
    std::thread::spawn(move || {
        match tts::Tts::new(tts::Backends::WinRt) {
            Ok(mut tts) => {
                match tts.voices() {
                    Ok(vs) => {
                        let names: Vec<String> = vs.iter().map(|v| v.name()).collect();
                        log_line(&dir, &format!("TTS 可用语音 {} 个: {names:?}", vs.len()));
                    }
                    Err(e) => log_error(&dir, &format!("TTS voices 查询失败: {e}")),
                }
                match tts.voice() {
                    Ok(Some(v)) => log_line(&dir, &format!("TTS 当前语音: {}", v.name())),
                    Ok(None) => log_line(&dir, "TTS 当前语音: 无"),
                    Err(e) => log_error(&dir, &format!("TTS 当前语音查询失败: {e}")),
                }
                match tts.speak(text, true) {
                    Ok(_) => log_line(&dir, "TTS 播放已提交"),
                    Err(e) => log_error(&dir, &format!("TTS 播放失败: {e}")),
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
