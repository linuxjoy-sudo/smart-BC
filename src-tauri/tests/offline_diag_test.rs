use smart_bc::asr::whisper::Transcriber;

#[test]
fn offline_transcribe_xiaobei() {
    let model = "/mnt/c/Users/HuangYi/AppData/Local/smartbc/models/ggml-base.bin";
    let t = Transcriber::new(std::path::Path::new(model)).expect("load model");
    let out = t.transcribe(std::path::Path::new("/tmp/xiaobei.wav")).expect("transcribe");
    println!("[base] 小贝小贝.m4a => {out:?}");
    let contains = smart_bc::voice::wake::contains_wake_word(&out, "小贝小贝");
    println!("[base] contains_wake_word = {contains}");
    let t2 = smart_bc::asr::whisper::Transcriber::new(
        std::path::Path::new("/mnt/c/Users/HuangYi/AppData/Local/smartbc/models/ggml-small.bin"))
        .expect("load small");
    let out2 = t2.transcribe(std::path::Path::new("/tmp/xiaobei.wav")).expect("transcribe small");
    println!("[small] 小贝小贝.m4a => {out2:?}");
    let contains2 = smart_bc::voice::wake::contains_wake_word(&out2, "小贝小贝");
    println!("[small] contains_wake_word = {contains2}");
}
