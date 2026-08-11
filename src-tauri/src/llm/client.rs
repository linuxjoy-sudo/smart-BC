use super::provider::{LlmError, LlmProvider};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

pub struct DeepSeekClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    http: reqwest::blocking::Client,
}

impl DeepSeekClient {
    pub fn new(api_key: &str) -> Self {
        Self::with_base("https://api.deepseek.com".into(), api_key, "deepseek-chat")
    }

    pub fn with_base(base_url: String, api_key: &str, model: &str) -> Self {
        Self {
            base_url,
            api_key: api_key.to_string(),
            model: model.to_string(),
            http: reqwest::blocking::Client::new(),
        }
    }
}

impl LlmProvider for DeepSeekClient {
    fn chat_json(&self, system: &str, user: &str) -> Result<Value, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "response_format": {"type": "json_object"},
            "temperature": 0.2
        });
        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .map_err(LlmError::Http)?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(LlmError::Status { code: status.as_u16(), body: text });
        }
        let json: Value = resp.json().map_err(LlmError::Http)?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(LlmError::EmptyContent)?;
        serde_json::from_str(content).map_err(|e| LlmError::InvalidJson(e.to_string()))
    }
}
