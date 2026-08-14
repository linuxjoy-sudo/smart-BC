use crate::llm::provider::{LlmError, LlmProvider};
use crate::memory::types::MemoryExtraction;

pub fn build_extract_prompt(transcript: &str) -> String {
    format!(
        r#"你是个人助理的记忆抽取器。从用户的语音转写文本中抽取结构化记忆，只输出合法 JSON，不要任何多余文字。
输出格式（缺失项给空数组或 null）：
{{
  "people": [{{"name": "人名", "relation": "关系(可选)", "note": "备注(可选)"}}],
  "reminders": [{{"content": "待办事项核心，去掉'提醒我/帮我/记得'等前缀（如'提醒我喝水'→'喝水'；'帮我记得买牛奶'→'买牛奶'）", "due": "时间表达：直接抄录用户说的相对时间原文（如'3分钟后'、'明天早上9点'、'后天下午3点'、'周五晚上'），没有明确时间则 null"}}],
  "preferences": [{{"topic": "偏好主题", "value": "偏好内容"}}],
  "episode": {{"summary": "本次事件摘要", "people": ["人物"], "place": "地点或 null"}}
}}

转写文本：
{transcript}"#,
        transcript = transcript
    )
}

/// 清洗提醒内容：去掉"提醒我/提醒你/帮我/记得"等前缀，得到待办事项核心。
pub fn clean_reminder_content(s: &str) -> String {
    let t = s.trim();
    for p in ["提醒我", "提醒你", "帮我提醒", "帮我", "记得提醒", "记得", "请提醒", "提醒"] {
        if let Some(rest) = t.strip_prefix(p) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return clean_reminder_content(rest);
            }
        }
    }
    t.to_string()
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
