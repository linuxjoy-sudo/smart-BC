pub fn contains_wake_word(text: &str, wake_word: &str) -> bool {
    if text.is_empty() || wake_word.is_empty() { return false; }
    // whisper 转写可能插入标点/空白（如"小贝﹑小贝﹑"），过滤后做连续子串匹配
    let compact: String = text.chars().filter(|c| c.is_alphanumeric()).collect();
    let key: String = wake_word.chars().filter(|c| c.is_alphanumeric()).collect();
    if key.is_empty() { return false; }
    compact.contains(&key)
}
