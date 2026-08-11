use crate::llm::provider::{LlmError, LlmProvider};
use crate::memory::types::MemoryExtraction;

pub fn build_extract_prompt(transcript: &str) -> String {
    format!(
        r#"你是个人助理的记忆抽取器。从用户的语音转写文本中抽取结构化记忆，只输出合法 JSON，不要任何多余文字。
输出格式（缺失项给空数组或 null）：
{{
  "people": [{{"name": "人名", "relation": "关系(可选)", "note": "备注(可选)"}}],
  "reminders": [{{"content": "承诺/待办内容", "due": "截止时间，ISO 日期或 null"}}],
  "preferences": [{{"topic": "偏好主题", "value": "偏好内容"}}],
  "episode": {{"summary": "本次事件摘要", "people": ["人物"], "place": "地点或 null"}}
}}

转写文本：
{transcript}"#,
        transcript = transcript
    )
}

pub fn parse_extraction(raw_json: &str) -> Result<MemoryExtraction, String> {
    let v: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|e| format!("非法 JSON: {e}"))?;
    let obj = v.as_object().ok_or("JSON 不是对象")?;
    let mut ext = MemoryExtraction::default();
    if let Some(p) = obj.get("people") {
        ext.people = serde_json::from_value(p.clone()).unwrap_or_default();
    }
    if let Some(r) = obj.get("reminders") {
        ext.reminders = serde_json::from_value(r.clone()).unwrap_or_default();
    }
    if let Some(p) = obj.get("preferences") {
        ext.preferences = serde_json::from_value(p.clone()).unwrap_or_default();
    }
    if let Some(e) = obj.get("episode") {
        if !e.is_null() {
            ext.episode = serde_json::from_value(e.clone()).ok();
        }
    }
    Ok(ext)
}

pub fn extract_from_transcript(
    provider: &dyn LlmProvider,
    transcript: &str,
) -> Result<MemoryExtraction, LlmError> {
    let raw = provider.chat_json(
        "你输出严格 JSON，不要 Markdown 代码块。",
        &build_extract_prompt(transcript),
    )?;
    parse_extraction(&raw.to_string()).map_err(LlmError::InvalidJson)
}
