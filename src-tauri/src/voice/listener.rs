use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct RingBuffer {
    data: VecDeque<f32>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity_secs: u32, sample_rate: u32) -> Self {
        Self { data: VecDeque::new(), capacity: (capacity_secs as usize) * sample_rate as usize }
    }

    pub fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            if self.data.len() >= self.capacity {
                self.data.pop_front();
            }
            self.data.push_back(s);
        }
    }

    pub fn snapshot(&self) -> Vec<f32> {
        self.data.iter().copied().collect()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

pub struct Listener {
    stream: cpal::Stream,
    pub buffer: Arc<Mutex<RingBuffer>>,
    pub sample_rate: u32,
}

impl Listener {
    pub fn start(device_index: Option<usize>, buffer_secs: u32) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = match device_index {
            Some(idx) => {
                let devices: Vec<_> = host
                    .input_devices()
                    .map_err(|e| format!("list devices: {e}"))?
                    .collect();
                devices
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| "invalid device index".to_string())?
            }
            None => host
                .default_input_device()
                .ok_or_else(|| "no input device".to_string())?,
        };
        let config = device
            .default_input_config()
            .map_err(|e| format!("input config: {e}"))?;
        let sample_rate = config.sample_rate();
        let buffer = Arc::new(Mutex::new(RingBuffer::new(buffer_secs, sample_rate)));
        let buf_cb = Arc::clone(&buffer);
        let stream = device
            .build_input_stream(
                config.into(),
                move |data: &[f32], _| {
                    if let Ok(mut b) = buf_cb.lock() {
                        b.push(data);
                    }
                },
                move |err| eprintln!("voice listener stream error: {err}"),
                None,
            )
            .map_err(|e| format!("build stream: {e}"))?;
        stream.play().map_err(|e| format!("play: {e}"))?;
        Ok(Self { stream, buffer, sample_rate })
    }

    pub fn stop(&self) -> Result<(), String> {
        self.stream.pause().map_err(|e| e.to_string())
    }
}
