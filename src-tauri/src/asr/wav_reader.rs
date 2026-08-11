use crate::audio::wav::read_f32_wav;
use std::path::Path;

pub fn read_any_wav(path: &Path) -> Result<(u32, Vec<f32>), String> {
    read_f32_wav(path).map_err(|e| e.to_string())
}
