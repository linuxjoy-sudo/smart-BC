use pinyin::{Pinyin, ToPinyin};

fn pinyin_key(text: &str) -> String {
    text.to_pinyin().flatten().map(Pinyin::plain).collect()
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
}
