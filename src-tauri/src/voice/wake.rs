pub fn contains_wake_word(text: &str, wake_word: &str) -> bool {
    if text.is_empty() || wake_word.is_empty() { return false; }
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let key: String = wake_word.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains(&key)
}
