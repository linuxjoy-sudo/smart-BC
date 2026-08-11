use hound::{WavSpec, WavWriter};
use std::path::Path;

#[derive(Debug)]
pub struct AudioError(pub String);

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for AudioError {}

pub fn write_f32_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
    if samples.is_empty() {
        return Err(AudioError("samples are empty".into()));
    }
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| AudioError(format!("create wav: {e}")))?;
    for &s in samples {
        writer
            .write_sample(s)
            .map_err(|e| AudioError(format!("write sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| AudioError(format!("finalize wav: {e}")))
}

pub fn read_f32_wav(path: &Path) -> Result<(u32, Vec<f32>), AudioError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| AudioError(format!("open wav: {e}")))?;
    let spec = reader.spec();
    if spec.channels != 1 {
        return Err(AudioError("expected mono wav".into()));
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            reader.samples::<f32>().collect::<Result<_, _>>().map_err(|e| AudioError(e.to_string()))?
        }
        hound::SampleFormat::Int => {
            let max = 2f64.powi(spec.bits_per_sample as i32 - 1);
            reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AudioError(e.to_string()))?
                .into_iter()
                .map(|s| s as f64 / max)
                .map(|s| s as f32)
                .collect()
        }
    };
    Ok((spec.sample_rate, samples))
}
