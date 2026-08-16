use std::path::Path;

/// 音频源抽象：run_loop 从 feed 取样本，支持真实麦克风（CpalFeed）与测试 wav（WavFeed）。
/// None 表示音频流结束（测试用，真实 feed 永不结束）。
pub trait AudioFeed {
    fn sample_rate(&self) -> u32;
    fn next_samples(&mut self, max_frames: usize) -> Option<Vec<f32>>;
}

/// 真实麦克风 feed：包装 Listener，每次取 RingBuffer 当前快照。
pub struct CpalFeed {
    pub listener: super::listener::Listener,
}

impl AudioFeed for CpalFeed {
    fn sample_rate(&self) -> u32 {
        self.listener.sample_rate
    }

    fn next_samples(&mut self, _max_frames: usize) -> Option<Vec<f32>> {
        let mut b = self.listener.buffer.lock().unwrap();
        let snap = b.snapshot();
        b.clear();
        Some(snap)
    }
}

/// 测试 feed：按块吐出 wav 样本，末尾补静音触发断句，耗尽返回 None。
pub struct WavFeed {
    samples: Vec<f32>,
    pos: usize,
    sample_rate: u32,
    trailing_silence: usize,
}

impl WavFeed {
    pub fn from_wav(path: &Path, trailing_silence_ms: u32) -> Result<Self, String> {
        let (rate, samples) = crate::asr::wav_reader::read_any_wav(path).map_err(|e| e.to_string())?;
        let silence = (rate * trailing_silence_ms / 1000) as usize;
        Ok(Self {
            samples,
            pos: 0,
            sample_rate: rate,
            trailing_silence: silence,
        })
    }
}

impl AudioFeed for WavFeed {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn next_samples(&mut self, max_frames: usize) -> Option<Vec<f32>> {
        let mut out = Vec::new();
        while out.len() < max_frames && self.pos < self.samples.len() {
            let n = (max_frames - out.len()).min(self.samples.len() - self.pos);
            out.extend_from_slice(&self.samples[self.pos..self.pos + n]);
            self.pos += n;
        }
        if out.len() < max_frames && self.trailing_silence > 0 {
            let n = (max_frames - out.len()).min(self.trailing_silence);
            out.extend(vec![0.0; n]);
            self.trailing_silence -= n;
        }
        if out.is_empty() && self.pos >= self.samples.len() && self.trailing_silence == 0 {
            None
        } else if out.is_empty() {
            Some(Vec::new())
        } else {
            Some(out)
        }
    }
}
