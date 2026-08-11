use smart_bc::asr;

#[test]
fn mono_16k_passthrough() {
    let input: Vec<f32> = (0..1600).map(|i| (i as f32) / 1600.0).collect();
    let out = asr::pcm::to_mono_f16k(16000, &input);
    assert_eq!(out.len(), 1600);
    assert!((out[0] - input[0]).abs() < 1e-6);
}

#[test]
fn downsample_48k_to_16k() {
    let input: Vec<f32> = vec![0.0; 4800];
    let out = asr::pcm::to_mono_f16k(48000, &input);
    assert_eq!(out.len(), 1600); // 4800 / 3
}

#[test]
fn model_path_is_under_data_dir() {
    let p = asr::model::model_path(std::path::Path::new("/tmp/appdata"));
    assert!(p.ends_with("models/ggml-small.bin"));
}
