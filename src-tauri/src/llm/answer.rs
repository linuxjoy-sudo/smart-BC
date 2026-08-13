use crate::db::memories::{PersonRow, PreferenceRow};
use crate::db::search::SearchHit;
use crate::llm::provider::{LlmError, LlmProvider};

pub fn build_answer_prompt(
    question: &str,
    hits: &[SearchHit],
    people: &[PersonRow],
    prefs: &[PreferenceRow],
) -> String {
    let mut evidence = String::new();
    for h in hits {
        evidence.push_str(&format!("[对话 #{}] {}\n", h.conversation_id, h.snippet));
    }
    let mut people_str = String::new();
    for p in people {
        people_str.push_str(&format!("- {}（关系:{}, 备注:{}）\n", p.name, p.relation, p.note));
    }
    let mut prefs_str = String::new();
    for pr in prefs {
        prefs_str.push_str(&format!("- {}: {}\n", pr.topic, pr.value));
    }
    format!(
        r#"你是用户的私人记忆助理。根据提供的记忆证据回答用户问题。
要求：
1. 只依据提供的证据回答；证据不足时明确说"我还没有这方面的记忆"。
2. 引用相关对话时标注其编号，如（对话 #3）。
3. 回答用中文，简洁具体。

用户问题：{question}

相关对话记录：
{evidence}
已记住的人脉：
{people_str}
已记住的偏好：
{prefs_str}"#
    )
}

pub fn answer_question(
    provider: &dyn LlmProvider,
    question: &str,
    hits: &[SearchHit],
    people: &[PersonRow],
    prefs: &[PreferenceRow],
) -> Result<String, LlmError> {
    let prompt = build_answer_prompt(question, hits, people, prefs);
    // DeepSeek json_object 要求 prompt 含 "json" 字样；约定输出 {"answer": "..."} 并解析
    let v = provider.chat_json(
        "你是记忆助理。请以 JSON 格式输出回答：{\"answer\": \"回答文本\"}，只输出合法 JSON，不要 Markdown 代码块。",
        &prompt,
    )?;
    Ok(v.get("answer")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            v.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| v.to_string())
        }))
}
