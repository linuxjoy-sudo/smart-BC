use smart_bc::audio::wav::{read_f32_wav, write_f32_wav};

#[test]
fn wav_roundtrip_preserves_samples() {
    let dir = std::env::temp_dir().join("smartbc_wav_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roundtrip.wav");
    let samples: Vec<f32> = (0..8000).map(|i| (i as f32 / 8000.0 * 3.14159).sin()).collect();
    write_f32_wav(&path, &samples, 16000).unwrap();
    let (rate, read_back) = read_f32_wav(&path).unwrap();
    assert_eq!(rate, 16000);
    assert_eq!(read_back.len(), samples.len());
    for (a, b) in samples.iter().zip(read_back.iter()) {
        assert!((a - b).abs() < 1e-4, "sample mismatch {a} vs {b}");
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn wav_rejects_empty() {
    let dir = std::env::temp_dir().join("smartbc_wav_test");
    let path = dir.join("empty.wav");
    let err = write_f32_wav(&path, &[], 16000).unwrap_err();
    assert!(err.to_string().contains("empty"));
}
