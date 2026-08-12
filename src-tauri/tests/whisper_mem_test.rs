use smart_bc::asr::whisper::Transcriber;

#[test]
fn transcribe_samples_errors_without_model() {
    let t = Transcriber::new(std::path::Path::new("/nonexistent/model.bin"));
    assert!(t.is_err());
}
