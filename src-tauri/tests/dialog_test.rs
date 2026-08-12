use smart_bc::voice::dialog::{transition, DialogEvent, DialogState};

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
