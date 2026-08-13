use smart_bc::voice::wake::contains_wake_word;

#[test]
fn matches_exact() {
    assert!(contains_wake_word("小贝小贝，明天几点开会", "小贝小贝"));
}

#[test]
fn matches_with_whitespace() {
    assert!(contains_wake_word("小贝 小贝 帮我查日程", "小贝小贝"));
}

#[test]
fn no_match_without_wake() {
    assert!(!contains_wake_word("明天几点开会", "小贝小贝"));
}

#[test]
fn no_match_partial() {
    assert!(!contains_wake_word("小贝，你好", "小贝小贝"));
}

#[test]
fn whitespace_only_wake_word_matches_nothing() {
    assert!(!contains_wake_word("小贝小贝，明天几点开会", "   "));
    assert!(!contains_wake_word("小贝小贝，明天几点开会", "\t \n"));
}

#[test]
fn matches_with_punctuation_between_chars() {
    assert!(contains_wake_word("小贝﹑小贝﹑", "小贝小贝"));
    assert!(contains_wake_word("小贝，小贝！", "小贝小贝"));
}

#[test]
fn punctuation_only_wake_word_matches_nothing() {
    assert!(!contains_wake_word("小贝小贝，明天几点开会", "﹑﹑"));
    assert!(!contains_wake_word("小贝小贝，明天几点开会", "，。"));
}
