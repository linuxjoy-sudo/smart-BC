use crate::asr::pcm::to_mono_f16k;
use crate::asr::wav_reader::read_any_wav;
use std::path::Path;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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

    pub fn transcribe(&self, wav_path: &Path) -> Result<String, String> {
        let (rate, samples) = read_any_wav(wav_path).map_err(|e| e.to_string())?;
        self.transcribe_samples(rate, &samples)
    }

    pub fn transcribe_samples(&self, rate: u32, samples: &[f32]) -> Result<String, String> {
        let mono = to_mono_f16k(rate, samples);
        if mono.is_empty() {
            return Err("音频为空".into());
        }
        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("zh"));
        params.set_n_threads(4);
        params.set_translate(false);
        state
            .full(params, &mono)
            .map_err(|e| format!("whisper run: {e}"))?;
        let mut text = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(seg) = state.get_segment(i) {
                text.push_str(seg.to_str().map_err(|e| e.to_string())?);
            }
        }
        Ok(text.trim().to_string())
    }
}
