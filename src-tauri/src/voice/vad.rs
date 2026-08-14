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
    frames_above: u32,
    frames_below: u32,
}

impl EnergyVad {
    pub fn new(threshold: f32, frame_len: usize) -> Self {
        Self {
            threshold,
            speaking: false,
            frame: Vec::with_capacity(frame_len),
            frame_len,
            frames_above: 0,
            frames_below: 0,
        }
    }

    pub fn feed(&mut self, samples: &[f32]) -> bool {
        self.frame.extend_from_slice(samples);
        while self.frame.len() >= self.frame_len {
            let chunk: Vec<f32> = self.frame.drain(..self.frame_len).collect();
            let e = rms(&chunk);
            if e > self.threshold {
                self.frames_above += 1;
                self.frames_below = 0;
            } else {
                self.frames_above = 0;
                self.frames_below += 1;
            }
            if self.frames_above >= 3 {
                self.speaking = true;
            } else if self.frames_below >= 10 {
                self.speaking = false;
            }
        }
        self.speaking
    }
}
