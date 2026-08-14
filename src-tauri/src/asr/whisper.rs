use crate::asr::pcm::to_mono_f16k;
use crate::asr::wav_reader::read_any_wav;
use std::path::Path;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

#[derive(Clone)]
pub struct Transcriber {
    ctx: Arc<WhisperContext>,
}

impl Transcriber {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("load whisper model: {e}"))?;
        Ok(Self { ctx: Arc::new(ctx) })
    }

    pub fn new_state(&self) -> Result<WhisperState, String> {
        self.ctx.create_state().map_err(|e| e.to_string())
    }

    pub fn transcribe(&self, wav_path: &Path) -> Result<String, String> {
        let (rate, samples) = read_any_wav(wav_path).map_err(|e| e.to_string())?;
        self.transcribe_samples(rate, &samples)
    }

    pub fn transcribe_samples(&self, rate: u32, samples: &[f32]) -> Result<String, String> {
        let mut state = self.new_state()?;
        self.transcribe_with_state(&mut state, rate, samples)
    }

    pub fn transcribe_with_state(
        &self,
        state: &mut WhisperState,
        rate: u32,
        samples: &[f32],
    ) -> Result<String, String> {
        let mono = to_mono_f16k(rate, samples);
        if mono.is_empty() {
            return Err("音频为空".into());
        }
        let mono = normalize_volume(&mono);
        if mono.is_empty() {
            return Err("音频为空".into());
        }
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("zh"));
        params.set_n_threads(4);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_no_speech_thold(0.9);
        params.set_temperature(0.0);
        params.set_initial_prompt("以下是简体中文对话：");
        state
            .full(params, &mono)
            .map_err(|e| format!("whisper run: {e}"))?;
        let mut text = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(seg) = state.get_segment(i) {
                text.push_str(seg.to_str().map_err(|e| e.to_string())?);
            }
        }
        Ok(crate::asr::zh::to_simplified(text.trim()).trim().to_string())
    }
}

/// 音量归一化：弱麦克风输入（RMS 偏低）放大到合理水平，改善 whisper 识别。
fn normalize_volume(mono: &[f32]) -> Vec<f32> {
    const TARGET_RMS: f32 = 0.2;
    const MAX_GAIN: f32 = 8.0;
    if mono.is_empty() {
        return Vec::new();
    }
    let sum: f32 = mono.iter().map(|s| s * s).sum();
    let rms = (sum / mono.len() as f32).sqrt();
    if rms < 0.001 {
        return mono.to_vec();
    }
    let gain = (TARGET_RMS / rms).min(MAX_GAIN);
    mono.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_weak_signal() {
        let v = vec![0.03f32; 4800];
        let out = normalize_volume(&v);
        let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
        assert!(rms > 0.05, "弱信号应被放大，实际 rms={rms}");
    }

    #[test]
    fn silence_stays_silent() {
        let v = vec![0.0f32; 4800];
        let out = normalize_volume(&v);
        assert_eq!(out, v);
    }
}
