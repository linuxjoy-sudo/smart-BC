/// 高质量重采样到 16kHz 单声道（whisper 要求 16kHz f32 单声道）。
/// 使用 rubato sinc 插值（BlackmanHarris 窗），比线性插值保留更多高频信息，改善识别。
pub fn to_mono_f16k(sample_rate: u32, samples: &[f32]) -> Vec<f32> {
    use rubato::audioadapter_buffers::owned::InterleavedOwned;
    use rubato::{
        Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
        WindowFunction,
    };
    const TARGET: f64 = 16000.0;
    if sample_rate == 0 {
        return Vec::new();
    }
    if (sample_rate as f64 - TARGET).abs() < 1.0 {
        return samples.to_vec();
    }
    let ratio = TARGET / sample_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let chunk = samples.len().max(1);
    let mut resampler =
        match Async::<f32>::new_sinc(ratio, 2.0, &params, chunk, 1, FixedAsync::Input) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
    let input = match InterleavedOwned::<f32>::new_from(samples.to_vec(), 1, chunk) {
        Ok(i) => i,
        Err(_) => return Vec::new(),
    };
    match resampler.process(&input, 0, None) {
        Ok(out) => out.take_data(),
        Err(_) => Vec::new(),
    }
}
