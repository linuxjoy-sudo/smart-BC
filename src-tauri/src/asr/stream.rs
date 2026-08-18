use sherpa_onnx_sys::FeatureConfig;
use sherpa_onnx::{
    OnlineModelConfig, OnlineParaformerModelConfig, OnlineRecognizer, OnlineRecognizerConfig,
    OnlineStream,
};
use std::path::Path;

/// 流式 ASR：sherpa-onnx paraformer，音频块喂入 → partial 文本（边说边识别）。
pub struct StreamingAsr {
    recognizer: OnlineRecognizer,
    stream: OnlineStream,
}

impl StreamingAsr {
    pub fn new(model_dir: &Path) -> Result<Self, String> {
        let encoder = model_dir.join("encoder.int8.onnx");
        let decoder = model_dir.join("decoder.int8.onnx");
        let tokens = model_dir.join("tokens.txt");
        if !encoder.exists() || !decoder.exists() || !tokens.exists() {
            return Err(format!(
                "流式模型缺失（需要 encoder/decoder/tokens，目录：{}）",
                model_dir.display()
            ));
        }
        let config = OnlineRecognizerConfig {
            feat_config: FeatureConfig {
                sample_rate: 16000,
                feature_dim: 80,
            },
            model_config: OnlineModelConfig {
                paraformer: OnlineParaformerModelConfig {
                    encoder: Some(encoder.to_string_lossy().into_owned()),
                    decoder: Some(decoder.to_string_lossy().into_owned()),
                    ..Default::default()
                },
                tokens: Some(tokens.to_string_lossy().into_owned()),
                ..Default::default()
            },
            decoding_method: Some("greedy_search".into()),
            ..Default::default()
        };
        let recognizer = OnlineRecognizer::create(&config)
            .ok_or_else(|| "初始化流式识别器失败".to_string())?;
        let stream = recognizer.create_stream();
        Ok(Self { recognizer, stream })
    }

    /// 喂入音频块（16k f32），返回 partial 文本（可能变化），None 表示暂无结果。
    pub fn feed(&mut self, sample_rate: u32, samples: &[f32]) -> Result<Option<String>, String> {
        self.stream.accept_waveform(sample_rate as i32, samples);
        self.recognizer.decode(&self.stream);
        Ok(self.recognizer.get_result(&self.stream).map(|r| r.text.trim().to_string()))
    }

    /// 句尾：重置流提交当前句，返回 final 文本。
    pub fn finalize(&mut self) -> Option<String> {
        let text = self.recognizer.get_result(&self.stream).map(|r| r.text.trim().to_string());
        self.recognizer.reset(&self.stream);
        text.filter(|t| !t.is_empty())
    }

    pub fn is_ready(&self) -> bool {
        self.recognizer.is_ready(&self.stream)
    }
}
