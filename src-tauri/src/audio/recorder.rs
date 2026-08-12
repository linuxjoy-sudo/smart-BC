use crate::audio::wav::{write_f32_wav, AudioError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    pub index: usize,
    pub name: String,
}

pub fn list_input_devices() -> Result<Vec<AudioDeviceInfo>, AudioError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|e| AudioError(format!("list input devices: {e}")))?;
    let mut out = Vec::new();
    for (i, d) in devices.enumerate() {
        let name = d
            .description()
            .map(|desc| desc.name().to_string())
            .unwrap_or_else(|_| format!("设备 {i}"));
        out.push(AudioDeviceInfo { index: i, name });
    }
    Ok(out)
}

pub struct Recorder {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl Recorder {
    pub fn new(device_index: Option<usize>) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = match device_index {
            Some(idx) => {
                let devices: Vec<_> = host
                    .input_devices()
                    .map_err(|e| AudioError(format!("list devices: {e}")))?
                    .collect();
                devices
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| AudioError("invalid device index".into()))?
            }
            None => host
                .default_input_device()
                .ok_or_else(|| AudioError("no input device".into()))?,
        };
        let config = device
            .default_input_config()
            .map_err(|e| AudioError(format!("input config: {e}")))?;
        let sample_rate = config.sample_rate();
        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_cb = Arc::clone(&samples);
        let stream = device
            .build_input_stream(
                config.into(),
                move |data: &[f32], _| {
                    samples_cb.lock().unwrap().extend_from_slice(data);
                },
                move |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| AudioError(format!("build stream: {e}")))?;
        Ok(Self { stream, samples, sample_rate })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn start(&self) -> Result<(), AudioError> {
        self.samples.lock().unwrap().clear();
        self.stream.play().map_err(|e| AudioError(format!("play: {e}")))
    }

    pub fn stop_and_save(&self, path: &Path) -> Result<usize, AudioError> {
        self.stream.pause().map_err(|e| AudioError(format!("pause: {e}")))?;
        let samples: Vec<f32> = self.samples.lock().unwrap().clone();
        if samples.is_empty() {
            return Err(AudioError("no audio captured".into()));
        }
        write_f32_wav(path, &samples, self.sample_rate)?;
        Ok(samples.len())
    }
}
