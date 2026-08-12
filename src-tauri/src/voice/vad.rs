pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

pub struct EnergyVad {
    pub threshold: f32,
    pub speaking: bool,
    frame: Vec<f32>,
    frame_len: usize,
}

impl EnergyVad {
    pub fn new(threshold: f32, frame_len: usize) -> Self {
        Self { threshold, speaking: false, frame: Vec::with_capacity(frame_len), frame_len }
    }

    pub fn feed(&mut self, samples: &[f32]) -> bool {
        self.frame.extend_from_slice(samples);
        while self.frame.len() >= self.frame_len {
            let chunk: Vec<f32> = self.frame.drain(..self.frame_len).collect();
            let e = rms(&chunk);
            self.speaking = e > self.threshold;
        }
        self.speaking
    }
}
