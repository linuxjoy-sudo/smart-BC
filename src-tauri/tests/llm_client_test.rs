use httpmock::prelude::*;
use smart_bc::llm::client::DeepSeekClient;
use smart_bc::llm::provider::LlmProvider;

#[test]
fn chat_json_returns_content() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/chat/completions")
            .header("Authorization", "Bearer test-key");
        then.status(200).json_body(serde_json::json!({
            "choices": [{ "message": { "content": "{\"ok\":true}" } }]
        }));
    });
    let client = DeepSeekClient::with_base(server.base_url(), "test-key", "deepseek-chat");
    let result = client.chat_json("sys", "user").unwrap();
    assert_eq!(result["ok"], true);
    mock.assert();
}

#[test]
fn chat_json_maps_http_errors() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(429).json_body(serde_json::json!({"error": {"message": "rate limited"}}));
    });
    let client = DeepSeekClient::with_base(server.base_url(), "k", "m");
    let err = client.chat_json("sys", "user").unwrap_err();
    assert!(err.to_string().contains("429"), "got: {err}");
}

#[test]
fn chat_json_rejects_bad_json_content() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200).json_body(serde_json::json!({
            "choices": [{ "message": { "content": "not json" } }]
        }));
    });
    let client = DeepSeekClient::with_base(server.base_url(), "k", "m");
    let err = client.chat_json("sys", "user").unwrap_err();
    assert!(err.to_string().contains("JSON"));
}
