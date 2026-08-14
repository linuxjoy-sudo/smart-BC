use smart_bc::memory::extract::{build_extract_prompt, clean_reminder_content, parse_extraction};

#[test]
fn parse_full_extraction() {
    let raw = r#"{
      "people": [{"name": "张伟", "relation": "供应商", "note": "聊过预算"}],
      "reminders": [{"content": "周三交方案", "due": "2026-08-12"}],
      "preferences": [{"topic": "饮食", "value": "不吃香菜"}],
      "episode": {"summary": "和张伟讨论供应商预算", "people": ["张伟"], "place": "公司"}
    }"#;
    let parsed = parse_extraction(raw).unwrap();
    assert_eq!(parsed.people.len(), 1);
    assert_eq!(parsed.people[0].name, "张伟");
    assert_eq!(parsed.reminders[0].due.as_deref(), Some("2026-08-12"));
    assert_eq!(parsed.preferences[0].value, "不吃香菜");
    assert!(parsed.episode.is_some());
}

#[test]
fn parse_missing_fields_defaults_empty() {
    let raw = r#"{"people": [], "reminders": [], "preferences": []}"#;
    let parsed = parse_extraction(raw).unwrap();
    assert!(parsed.people.is_empty());
    assert!(parsed.reminders.is_empty());
    assert!(parsed.preferences.is_empty());
    assert!(parsed.episode.is_none());
}

#[test]
fn parse_invalid_json_errors() {
    let err = parse_extraction("not json").unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn prompt_contains_transcript_and_json_requirement() {
    let p = build_extract_prompt("周三交方案给张伟");
    assert!(p.contains("周三交方案给张伟"));
    assert!(p.contains("JSON"), "prompt: {p}");
}

#[test]
fn clean_reminder_strips_prefixes() {
    assert_eq!(clean_reminder_content("提醒我喝水"), "喝水");
    assert_eq!(clean_reminder_content("帮我记得买牛奶"), "买牛奶");
    assert_eq!(clean_reminder_content("提醒我提醒我喝水"), "喝水");
    assert_eq!(clean_reminder_content("去孙里"), "去孙里");
    assert_eq!(clean_reminder_content(""), "");
}

#[test]
fn prompt_guides_clean_content() {
    let p = build_extract_prompt("提醒我喝水");
    assert!(p.contains("提醒我喝水"), "prompt: {p}");
}
