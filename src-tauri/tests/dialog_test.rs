use smart_bc::voice::dialog::{silence_exceeded, transition, window_expired, DialogEvent, DialogState};

#[test]
fn idle_to_active_on_wake() {
    let s = transition(DialogState::Idle, DialogEvent::WakeWordHit);
    assert!(matches!(s, DialogState::Active));
}

#[test]
fn idle_stays_idle_without_wake() {
    let s = transition(DialogState::Idle, DialogEvent::VoiceStart);
    assert!(matches!(s, DialogState::Idle));
}

#[test]
fn active_to_processing_on_sentence_end() {
    let s = transition(DialogState::Active, DialogEvent::SentenceEnd);
    assert!(matches!(s, DialogState::Processing));
}

#[test]
fn processing_returns_to_active_on_success() {
    let s = transition(DialogState::Processing, DialogEvent::ProcessedOk);
    assert!(matches!(s, DialogState::Active));
}

#[test]
fn processing_returns_to_active_on_error() {
    let s = transition(DialogState::Processing, DialogEvent::ProcessedErr);
    assert!(matches!(s, DialogState::Active));
}

#[test]
fn active_times_out_to_idle() {
    let s = transition(DialogState::Active, DialogEvent::WindowTimeout);
    assert!(matches!(s, DialogState::Idle));
}

#[test]
fn window_expiry_after_30s() {
    assert!(window_expired(std::time::Duration::from_secs(31), std::time::Duration::from_secs(30)));
    assert!(!window_expired(std::time::Duration::from_secs(10), std::time::Duration::from_secs(30)));
}

#[test]
fn silence_limit_for_sentence_end() {
    assert!(silence_exceeded(1.6, 1.5));
    assert!(!silence_exceeded(0.5, 1.5));
}
