/// 线性插值重采样到 16kHz 单声道（whisper 要求 16kHz f32 单声道）。
/// 相比简单抽点，插值避免 48k→16k 混叠（aliasing），保留更多语音高频信息。
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
        let src = i as f64 * ratio;
        let s0 = src.floor() as usize;
        let frac = (src - s0 as f64) as f32;
        let a = samples.get(s0).copied().unwrap_or(0.0);
        let b = samples.get(s0 + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}
