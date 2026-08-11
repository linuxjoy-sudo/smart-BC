/// 线性重采样到 16kHz 单声道（whisper 要求 16kHz f32 单声道）。
pub fn to_mono_f16k(sample_rate: u32, samples: &[f32]) -> Vec<f32> {
    const TARGET: f64 = 16000.0;
    if sample_rate == 0 {
        return Vec::new();
    }
    if (sample_rate as f64 - TARGET).abs() < 1.0 {
        return samples.to_vec();
    }
    let ratio = sample_rate as f64 / TARGET;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_idx = (i as f64 * ratio) as usize;
        out.push(samples.get(src_idx).copied().unwrap_or(0.0));
    }
    out
}
