use smart_bc::voice::vad::{rms, EnergyVad};

#[test]
fn rms_of_silence_is_zero() {
    let samples = vec![0.0f32; 480];
    assert!(rms(&samples) < 0.0001);
}

#[test]
fn rms_of_constant_signal() {
    let samples = vec![0.5f32; 480];
    assert!((rms(&samples) - 0.5).abs() < 0.001);
}

#[test]
fn vad_requires_3_frames_to_onset() {
    let mut vad = EnergyVad::new(0.02, 480); // 10ms@48k
    let voice = vec![0.5f32; 480];
    assert!(!vad.feed(&voice)); // 第 1 帧
    assert!(!vad.feed(&voice)); // 第 2 帧
    assert!(vad.feed(&voice));  // 第 3 帧 → speaking
}

#[test]
fn vad_ignores_single_noise_frame() {
    let mut vad = EnergyVad::new(0.02, 480);
    let voice = vec![0.5f32; 480];
    let silence = vec![0.0f32; 480];
    // 单帧噪声 + 静音，不触发起音
    vad.feed(&voice);
    assert!(!vad.speaking);
    assert!(!vad.feed(&silence));
}

#[test]
fn vad_has_10_frame_hangover() {
    let mut vad = EnergyVad::new(0.02, 480);
    let voice = vec![0.5f32; 480];
    let silence = vec![0.0f32; 480];
    vad.feed(&voice);
    vad.feed(&voice);
    vad.feed(&voice); // 起音
    assert!(vad.speaking);
    for _ in 0..9 {
        vad.feed(&silence); // 9 帧静音仍在迟滞内
        assert!(vad.speaking);
    }
    assert!(!vad.feed(&silence)); // 第 10 帧 → 结束
}

#[test]
fn vad_bridges_syllable_gap() {
    let mut vad = EnergyVad::new(0.02, 480);
    let voice = vec![0.5f32; 480];
    let silence = vec![0.0f32; 480];
    vad.feed(&voice); vad.feed(&voice); vad.feed(&voice);
    // "小贝小贝" 音节间隙：5 帧静音（50ms）不结束
    for _ in 0..5 {
        vad.feed(&silence);
    }
    assert!(vad.speaking);
    // 后续语音继续
    assert!(vad.feed(&voice));
    assert!(vad.feed(&voice));
    assert!(vad.feed(&voice));
}
