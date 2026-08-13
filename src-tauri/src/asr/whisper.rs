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
        let mono = trim_leading_silence(&mono);
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

/// 按 10ms 帧 RMS 扫描，裁掉首段连续低于阈值的静音（缩短 encoder 输入）。
fn trim_leading_silence(mono: &[f32]) -> Vec<f32> {
    const FRAME: usize = 160; // 16kHz 10ms
    const THRESHOLD: f32 = 0.01;
    if mono.len() <= FRAME {
        return mono.to_vec();
    }
    let mut start = 0;
    while start + FRAME <= mono.len() {
        let frame = &mono[start..start + FRAME];
        let sum: f32 = frame.iter().map(|s| s * s).sum();
        if (sum / FRAME as f32).sqrt() > THRESHOLD {
            break;
        }
        start += FRAME;
    }
    if start >= mono.len() {
        return Vec::new();
    }
    mono[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_leading_silence_only() {
        let mut v = vec![0.0f32; 3200]; // 200ms 静音
        v.extend(vec![0.5f32; 1600]); // 100ms 语音
        let out = trim_leading_silence(&v);
        assert!(out.len() < v.len());
        assert!(out.iter().any(|&s| s > 0.4));
    }

    #[test]
    fn keeps_non_silent_unchanged() {
        let v = vec![0.5f32; 4800];
        let out = trim_leading_silence(&v);
        assert_eq!(out.len(), v.len());
    }

    #[test]
    fn all_silence_returns_empty() {
        let v = vec![0.0f32; 4800];
        assert!(trim_leading_silence(&v).is_empty());
    }
}
