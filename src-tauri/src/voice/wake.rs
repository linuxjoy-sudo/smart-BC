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

/// 模糊包含：wake_key 的字符按顺序在 text_key 中出现，允许跳过少量插入字符
/// （whisper 常插入"的/得/地"等助词，如"小贝的小贝"→xiaobeidexiaobei）。
/// 唤醒词之后的内容不限制（正常句子尾部）。
fn fuzzy_contains(text_key: &str, wake_key: &str, max_insert: usize) -> bool {
    let text_chars: Vec<char> = text_key.chars().collect();
    let wake_chars: Vec<char> = wake_key.chars().collect();
    let mut ti = 0;
    let mut skipped = 0;
    for &wc in &wake_chars {
        while ti < text_chars.len() && text_chars[ti] != wc {
            ti += 1;
            skipped += 1;
            if skipped > max_insert {
                return false;
            }
        }
        if ti >= text_chars.len() {
            return false;
        }
        ti += 1;
    }
    true
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
    fuzzy_contains(&text_key, &wake_key, 3)
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
