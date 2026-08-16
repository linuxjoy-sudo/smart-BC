use crate::app_state::AppState;
use rusqlite::Connection;

pub enum DeviceTarget {
    Internal,
    Headset,
}

pub enum SystemCommand {
    ListHistory,
    ListReminders,
    ListPeople,
    Status,
    SwitchDevice(DeviceTarget),
    None,
}

pub fn parse_system_command(text: &str) -> SystemCommand {
    let t = text.trim();
    if ["查看历史", "最近记录", "最近说了什么", "查看记录", "看历史"]
        .iter()
        .any(|k| t.contains(k))
    {
        SystemCommand::ListHistory
    } else if ["查看承诺", "提醒列表", "有什么提醒", "查看提醒", "待办"]
        .iter()
        .any(|k| t.contains(k))
    {
        SystemCommand::ListReminders
    } else if ["查看人脉", "认识谁", "看人脉", "人脉列表"]
        .iter()
        .any(|k| t.contains(k))
    {
        SystemCommand::ListPeople
    } else if ["助手状态", "模型状态", "设备状态", "工作状态"]
        .iter()
        .any(|k| t.contains(k))
    {
        SystemCommand::Status
    } else if ["内置麦克风", "用内置"].iter().any(|k| t.contains(k)) {
        SystemCommand::SwitchDevice(DeviceTarget::Internal)
    } else if ["用耳机", "换耳机", "用蓝牙"].iter().any(|k| t.contains(k)) {
        SystemCommand::SwitchDevice(DeviceTarget::Headset)
    } else {
        SystemCommand::None
    }
}

pub fn execute_system_command<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    conn: &Connection,
    cmd: SystemCommand,
) -> String {
    match cmd {
        SystemCommand::ListHistory => list_history(conn),
        SystemCommand::ListReminders => list_reminders(conn),
        SystemCommand::ListPeople => list_people(conn),
        SystemCommand::Status => status(state),
        SystemCommand::SwitchDevice(target) => switch_device(app, state, target),
        SystemCommand::None => String::new(),
    }
}

fn list_history(conn: &Connection) -> String {
    let rows = crate::commands::query::list_conversations_impl(conn).unwrap_or_default();
    if rows.is_empty() {
        return "还没有任何记录".into();
    }
    let mut parts = Vec::new();
    for (i, r) in rows.iter().take(3).enumerate() {
        let text = r.summary.as_deref().unwrap_or(&r.transcript);
        parts.push(format!("{}：{}", i + 1, text));
    }
    format!("最近记录，{}", parts.join("。"))
}

fn list_reminders(conn: &Connection) -> String {
    let rows = crate::db::reminders::list_reminders(conn).unwrap_or_default();
    let pending: Vec<_> = rows.iter().filter(|r| r.status == "pending").collect();
    if pending.is_empty() {
        return "目前没有待办提醒".into();
    }
    let mut parts = Vec::new();
    for (i, r) in pending.iter().take(5).enumerate() {
        let due = r.due_at.as_deref().unwrap_or("时间待定");
        parts.push(format!("{}：{}({})", i + 1, r.content, due));
    }
    format!("你有{}个待办，{}", pending.len(), parts.join("。"))
}

fn list_people(conn: &Connection) -> String {
    let rows = crate::db::memories::list_people(conn).unwrap_or_default();
    if rows.is_empty() {
        return "你还没有记录过任何人脉".into();
    }
    let mut parts = Vec::new();
    for (i, p) in rows.iter().take(5).enumerate() {
        parts.push(format!("{}：{}（{}）", i + 1, p.name, p.relation));
    }
    format!("你记录过{}个人，{}", rows.len(), parts.join("。"))
}

fn status(state: &AppState) -> String {
    let model_ok = crate::asr::model::model_path(&state.data_dir).exists();
    let device = crate::audio::recorder::list_input_devices()
        .ok()
        .and_then(|devs| {
            devs.iter()
                .find(|d| Some(d.index) == state_config(state).input_device)
                .map(|d| d.name.clone())
        })
        .unwrap_or_else(|| "系统默认".into());
    let conn = state.conn.lock().unwrap();
    let pending = crate::db::reminders::list_reminders(&conn)
        .unwrap_or_default()
        .iter()
        .filter(|r| r.status == "pending")
        .count();
    drop(conn);
    let model = if model_ok { "已加载" } else { "未加载" };
    format!("语音助手运行正常。模型{}，当前设备{}，有{}个待办提醒", model, device, pending)
}

fn state_config(state: &AppState) -> crate::config::Config {
    crate::config::load_config(&state.data_dir)
}

fn switch_device<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &AppState, target: DeviceTarget) -> String {
    let devices = match crate::audio::recorder::list_input_devices() {
        Ok(d) => d,
        Err(_) => return "获取设备列表失败".into(),
    };
    let keyword = match target {
        DeviceTarget::Internal => "麦克风",
        DeviceTarget::Headset => "耳机",
    };
    let found = devices
        .iter()
        .find(|d| d.name.contains(keyword))
        .or_else(|| {
            devices.iter().find(|d| {
                let n = d.name.to_lowercase();
                n.contains("buds") || n.contains("headset") || n.contains("bluetooth")
            })
        });
    let Some(dev) = found else {
        return format!("没有找到{}设备", keyword);
    };
    let mut cfg = crate::config::load_config(&state.data_dir);
    cfg.input_device = Some(dev.index);
    if let Err(e) = crate::config::save_config(&state.data_dir, &cfg) {
        return format!("保存配置失败：{e}");
    }
    if let Err(e) = crate::commands::voice::restart_listener(app, state) {
        return format!("切换设备失败：{e}");
    }
    format!("已切换到{}", dev.name)
}
