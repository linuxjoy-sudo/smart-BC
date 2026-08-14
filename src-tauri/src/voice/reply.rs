#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverAction {
    Notification,
    Voice,
    Both,
}

pub fn deliver_plan(reply_mode: &str) -> DeliverAction {
    match reply_mode {
        "voice" => DeliverAction::Voice,
        "both" => DeliverAction::Both,
        _ => DeliverAction::Notification,
    }
}

pub fn deliver_reply(
    app: &tauri::AppHandle,
    data_dir: &std::path::Path,
    reply_mode: &str,
    message: String,
) {
    use crate::voice::log::log_error;
    use tauri_plugin_notification::NotificationExt;

    match deliver_plan(reply_mode) {
        DeliverAction::Notification => {
            if let Err(ne) = app.notification().builder().title("SmartBC").body(&message).show() {
                log_error(data_dir, &format!("通知发送失败: {ne}"));
            }
        }
        DeliverAction::Voice => {
            crate::voice::tts::speak_async(message);
        }
        DeliverAction::Both => {
            if let Err(ne) = app.notification().builder().title("SmartBC").body(&message).show() {
                log_error(data_dir, &format!("通知发送失败: {ne}"));
            }
            crate::voice::tts::speak_async(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_notification_by_default() {
        assert_eq!(deliver_plan(""), DeliverAction::Notification);
        assert_eq!(deliver_plan("notification"), DeliverAction::Notification);
        assert_eq!(deliver_plan("unknown"), DeliverAction::Notification);
    }

    #[test]
    fn plan_voice() {
        assert_eq!(deliver_plan("voice"), DeliverAction::Voice);
    }

    #[test]
    fn plan_both() {
        assert_eq!(deliver_plan("both"), DeliverAction::Both);
    }
}
