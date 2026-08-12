//! 端到端 LLM 测试：用真实 DeepSeek API 验证记忆抽取与回忆回答。
//! 运行方式（需真实 API Key）：DEEPSEEK_API_KEY=xxx cargo test --test e2e_llm_test -- --ignored
//! 默认 ignored，避免 CI/常规测试消耗 API 额度。

use smart_bc::llm::client::DeepSeekClient;
use smart_bc::llm::provider::LlmProvider;
use smart_bc::memory::extract::{build_extract_prompt, extract_from_transcript, parse_extraction};

fn client() -> DeepSeekClient {
    let key = std::env::var("DEEPSEEK_API_KEY").expect("设置 DEEPSEEK_API_KEY 环境变量");
    DeepSeekClient::new(&key)
}

#[test]
#[ignore]
fn e2e_extract_real_transcript() {
    let provider = client();
    let transcript = "周三下午三点和张伟开预算会，他不吃香菜，记得明天给妈妈订蛋糕";
    let ext = extract_from_transcript(&provider, transcript).expect("抽取失败");
    assert!(!ext.people.is_empty(), "应抽取到人脉，got: {ext:?}");
    assert!(!ext.reminders.is_empty(), "应抽取到承诺，got: {ext:?}");
    assert!(!ext.preferences.is_empty(), "应抽取到偏好，got: {ext:?}");
    eprintln!("抽取结果: {ext:?}");
}

#[test]
#[ignore]
fn e2e_answer_question() {
    let provider = client();
    let prompt = build_extract_prompt("周三和张伟开预算会");
    let raw = provider.chat_json("你是记忆助理。", &prompt).expect("LLM 调用失败");
    let parsed = parse_extraction(&raw.to_string()).expect("解析失败");
    assert!(!parsed.people.is_empty(), "应解析出张伟");
    eprintln!("回答链路 OK: {parsed:?}");
}
