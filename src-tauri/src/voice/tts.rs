#[cfg(windows)]
pub fn speak_async(data_dir: &std::path::Path, text: String) {
    use crate::voice::log::log_error;
    let dir = data_dir.to_path_buf();
    std::thread::spawn(move || {
        match tts::Tts::new(tts::Backends::WinRt) {
            Ok(mut tts) => {
                if let Err(e) = tts.speak(text, true) {
                    log_error(&dir, &format!("TTS 播放失败: {e}"));
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
