use smart_bc::asr::stream::StreamingAsr;
use std::path::PathBuf;

#[test]
fn streaming_recognize_synthetic() {
    let model = PathBuf::from("/mnt/c/Users/HuangYi/AppData/Local/smartbc/models/sherpa-paraformer-zh");
    if !model.join("encoder.int8.onnx").exists() {
        eprintln!("SKIP: 模型未就绪");
        return;
    }
    let mut asr = match StreamingAsr::new(&model) {
        Ok(a) => a,
        Err(e) => { eprintln!("init 失败: {e}"); return; }
    };
    // 1 秒静音 + 释放模型（仅验证初始化/生命周期不崩）
    let silence: Vec<f32> = vec![0.0; 16000];
    let _ = asr.feed(16000, &silence);
    println!("feed 静音 OK");
    drop(asr);
    println!("drop OK");
}
