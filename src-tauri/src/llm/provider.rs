use serde_json::Value;

#[derive(Debug)]
pub enum LlmError {
    Http(reqwest::Error),
    Status { code: u16, body: String },
    InvalidJson(String),
    EmptyContent,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::Http(e) => write!(f, "网络错误: {e}"),
            LlmError::Status { code, body } => write!(f, "LLM 返回错误 {code}: {body}"),
            LlmError::InvalidJson(s) => write!(f, "LLM 返回 JSON 解析失败: {s}"),
            LlmError::EmptyContent => write!(f, "LLM 返回空内容"),
        }
    }
}

impl std::error::Error for LlmError {}

pub trait LlmProvider {
    fn chat_json(&self, system: &str, user: &str) -> Result<Value, LlmError>;
}
