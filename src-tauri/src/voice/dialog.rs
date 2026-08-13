#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogState {
    Idle,
    Active,
    Processing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogEvent {
    VoiceStart,
    WakeWordHit,
    SentenceEnd,
    WindowTimeout,
    ProcessedOk,
    ProcessedErr,
    MicUnavailable,
}

pub fn run_listener(app: tauri::AppHandle, state: crate::app_state::AppState) {
    use crate::commands::voice::voice_assistant_enabled;
    use std::time::Duration;

    loop {
        if !voice_assistant_enabled(&state.data_dir) {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
        let _ = &app;
    }
}

pub fn transition(state: DialogState, event: DialogEvent) -> DialogState {
    match (state, event) {
        (DialogState::Idle, DialogEvent::WakeWordHit) => DialogState::Active,
        (DialogState::Active, DialogEvent::SentenceEnd) => DialogState::Processing,
        (DialogState::Processing, DialogEvent::ProcessedOk) => DialogState::Active,
        (DialogState::Processing, DialogEvent::ProcessedErr) => DialogState::Active,
        (DialogState::Active, DialogEvent::WindowTimeout) => DialogState::Idle,
        (_, DialogEvent::MicUnavailable) => DialogState::Idle,
        _ => state,
    }
}
