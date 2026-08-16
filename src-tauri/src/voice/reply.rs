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

/// 对话输出抽象：run_loop 通过 sink 播报，真实实现包装 AppHandle，测试用 MockSink 记录。
pub trait DialogSink {
    fn deliver(&self, reply_mode: &str, message: String);
    fn notify_error(&self, body: &str);
    fn tts_playing(&self) -> bool;
}

pub struct AppSink<'a, R: tauri::Runtime> {
    pub app: &'a tauri::AppHandle<R>,
    pub data_dir: &'a std::path::Path,
}

impl<R: tauri::Runtime> DialogSink for AppSink<'_, R> {
    fn deliver(&self, reply_mode: &str, message: String) {
        deliver_reply(self.app, self.data_dir, reply_mode, message);
    }

    fn notify_error(&self, body: &str) {
        use crate::voice::log::log_error;
        use tauri_plugin_notification::NotificationExt;
        if let Err(ne) = self
            .app
            .notification()
            .builder()
            .title("SmartBC")
            .body(body)
            .show()
        {
            log_error(self.data_dir, &format!("通知发送失败: {ne}"));
        }
    }

    fn tts_playing(&self) -> bool {
        crate::voice::tts::tts_playing()
    }
}

pub fn deliver_reply<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
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
            crate::voice::tts::speak_async(data_dir, message);
        }
        DeliverAction::Both => {
            if let Err(ne) = app.notification().builder().title("SmartBC").body(&message).show() {
                log_error(data_dir, &format!("通知发送失败: {ne}"));
            }
            crate::voice::tts::speak_async(data_dir, message);
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
