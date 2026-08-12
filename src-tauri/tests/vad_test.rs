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
fn vad_detects_voice_and_silence() {
    let mut vad = EnergyVad::new(0.02, 480); // 10ms@48k
    let silence = vec![0.0f32; 480];
    let voice = vec![0.5f32; 480];
    assert!(!vad.feed(&silence));
    assert!(vad.feed(&voice));
    assert!(!vad.feed(&silence));
}
