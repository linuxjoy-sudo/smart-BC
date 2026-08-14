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

#[test]
fn matches_homophone_variants() {
    assert!(contains_wake_word("小杯小杯", "小贝小贝"));
    assert!(contains_wake_word("小辈小辈", "小贝小贝"));
    assert!(contains_wake_word("小备小备", "小贝小贝"));
    assert!(contains_wake_word("小北小北", "小贝小贝"));
    assert!(contains_wake_word("小贝小贝", "小杯小杯"));
}

#[test]
fn no_match_for_different_initial_consonant() {
    assert!(!contains_wake_word("小费小费", "小贝小贝"));
}

#[test]
fn matches_with_inserted_particle() {
    assert!(contains_wake_word("小贝的小贝", "小贝小贝"));
}

#[test]
fn no_match_when_too_many_inserted_chars() {
    assert!(!contains_wake_word("小贝的的小贝", "小贝小贝"));
    assert!(!contains_wake_word("小贝的的的小贝", "小贝小贝"));
    assert!(!contains_wake_word("小贝今天小贝", "小贝小贝"));
}

#[test]
fn no_match_when_wake_word_chars_reordered() {
    assert!(!contains_wake_word("贝小贝小", "小贝小贝"));
}
