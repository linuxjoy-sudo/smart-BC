use pinyin::{Pinyin, ToPinyin};

fn pinyin_key(text: &str) -> String {
    let raw: String = text.to_pinyin().flatten().map(Pinyin::plain).collect();
    normalize_aspiration(&raw)
}

/// 送气声母归一化：whisper 常混淆送气/不送气对立（b/p、d/t、g/k、j/q、zh/ch、z/c），
/// 统一映射到不送气，使"小沛小沛"(xiaopei) 也能匹配"小贝小贝"(xiaobei)。
fn normalize_aspiration(s: &str) -> String {
    s.replace("ch", "zh")
        .replace('c', "z")
        .replace('p', "b")
        .replace('t', "d")
        .replace('k', "g")
        .replace('q', "j")
}

pub fn contains_wake_word(text: &str, wake_word: &str) -> bool {
    if text.is_empty() || wake_word.is_empty() {
        return false;
    }
    let text_key = pinyin_key(text);
    let wake_key = pinyin_key(wake_word);
    if wake_key.is_empty() {
        return false;
    }
    text_key.contains(&wake_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinyin_key_merges_homophones() {
        assert_eq!(pinyin_key("小贝小贝"), pinyin_key("小杯小杯"));
        assert_eq!(pinyin_key("小贝小贝"), pinyin_key("小辈小辈"));
    }

    #[test]
    fn pinyin_key_skips_punctuation() {
        assert_eq!(pinyin_key("小贝﹑小贝"), "xiaobeixiaobei");
    }

    #[test]
    fn pinyin_key_empty_for_ascii() {
        assert_eq!(pinyin_key("Hey："), "");
    }

    #[test]
    fn matches_aspiration_confusion() {
        assert!(contains_wake_word("小沛小沛", "小贝小贝"));
        assert!(contains_wake_word("小培小培", "小贝小贝"));
    }

    #[test]
    fn aspiration_normalized_equality() {
        assert_eq!(pinyin_key("小沛小沛"), pinyin_key("小贝小贝"));
    }
}
