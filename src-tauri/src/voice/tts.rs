#[cfg(windows)]
pub fn speak_async(text: String) {
    std::thread::spawn(move || {
        if let Ok(mut tts) = tts::Tts::new(tts::Backends::WinRt) {
            let _ = tts.speak(text, true);
        }
    });
}

#[cfg(not(windows))]
pub fn speak_async(_text: String) {
    eprintln!("TTS 播报仅在 Windows 可用");
}
