use crate::app_state::AppState;
use rusqlite::Connection;

#[derive(Debug)]
pub enum DeviceTarget {
    Internal,
    Headset,
}

#[derive(Debug)]
pub enum SystemCommand {
    ListHistory,
    ListReminders,
    ListPeople,
    Status,
    SwitchDevice(DeviceTarget),
    LaunchApp(String),
    LaunchWith(String, String),
    Search(String),
    OpenUrl(String),
    Volume(f32),
    Mute(bool),
    PlayMusic,
    MediaPlayPause,
    MediaNext,
    MediaPrev,
    None,
}

fn strip_verb(t: &str) -> &str {
    for v in ["打开", "启动", "开启", "运行", "帮我打开", "请打开"] {
        if let Some(rest) = t.strip_prefix(v) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    t
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
    } else if t.contains("用") && (t.contains("打开") || t.contains("开启")) {
        // "用记事本打开 D:\笔记.txt"
        let (app_part, target) = t.split_once("打开").or_else(|| t.split_once("开启")).unwrap_or(("", ""));
        let app = app_part.trim_start_matches("用").trim().to_string();
        let target = target.trim().to_string();
        if !app.is_empty() && !target.is_empty() {
            SystemCommand::LaunchWith(app, target)
        } else {
            SystemCommand::None
        }
    } else if t.starts_with("搜索") || t.starts_with("查一下") || t.starts_with("帮我搜") {
        let q = strip_verb(t)
            .trim_start_matches("搜索")
            .trim_start_matches("查一下")
            .trim_start_matches("帮我搜")
            .trim()
            .to_string();
        if !q.is_empty() {
            SystemCommand::Search(q)
        } else {
            SystemCommand::None
        }
    } else if t.contains("静音") {
        SystemCommand::Mute(true)
    } else if t.contains("取消静音") || t.contains("恢复声音") || t.contains("解除静音") {
        SystemCommand::Mute(false)
    } else if t.contains("下一首") {
        SystemCommand::MediaNext
    } else if t.contains("上一首") {
        SystemCommand::MediaPrev
    } else if t.contains("来点音乐")
        || t.contains("播放音乐")
        || t.contains("放音乐")
        || t.contains("放歌")
        || t.contains("放首")
        || t.contains("放点")
        || t.contains("来首")
        || t.contains("听歌")
    {
        SystemCommand::PlayMusic
    } else if t.contains("播放") || t.contains("暂停") || t.contains("继续") {
        SystemCommand::MediaPlayPause
    } else if t.contains("音量") {
        // "调高音量" / "调低音量" / "音量调到50" / "音量50"
        if t.contains("调高") || t.contains("大点") {
            SystemCommand::Volume(0.1)
        } else if t.contains("调低") || t.contains("小点") {
            SystemCommand::Volume(-0.1)
        } else if let Some(n) = extract_volume_number(t) {
            SystemCommand::Volume(n)
        } else {
            SystemCommand::None
        }
    } else if t.contains("网址") || {
        let u = strip_verb(t);
        (["打开", "启动", "开启"].iter().any(|v| t.contains(v)))
            && u.contains('.')
            && !u.contains(' ')
            && !u.contains('：')
    } {
        let url = strip_verb(t)
            .trim_start_matches("网址")
            .trim_start_matches("网站")
            .trim()
            .to_string();
        if !url.is_empty() {
            SystemCommand::OpenUrl(url)
        } else {
            SystemCommand::None
        }
    } else if ["打开", "启动", "开启", "运行"]
        .iter()
        .any(|v| t.starts_with(v) || t.starts_with(&format!("帮我{v}")) || t.starts_with(&format!("请{v}")))
    {
        let name = strip_verb(t).to_string();
        if !name.is_empty() {
            SystemCommand::LaunchApp(name)
        } else {
            SystemCommand::None
        }
    } else {
        SystemCommand::None
    }
}

fn extract_volume_number(t: &str) -> Option<f32> {
    // "音量调到50" / "音量50" / "五十"（中文数字）
    let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    if let Ok(n) = digits.parse::<u32>() {
        if (0..=100).contains(&n) {
            return Some(n as f32 / 100.0);
        }
    }
    None
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
        SystemCommand::LaunchApp(name) => launch_app(state, &name),
        SystemCommand::LaunchWith(app_name, target) => launch_with(state, &app_name, &target),
        SystemCommand::Search(q) => crate::voice::launch::search(&q),
        SystemCommand::OpenUrl(url) => crate::voice::launch::open_url(&url),
        SystemCommand::Volume(scale) => set_volume_cmd(scale),
        SystemCommand::Mute(mute) => set_mute_cmd(mute),
        SystemCommand::PlayMusic => play_music(state),
        SystemCommand::MediaPlayPause => media_cmd("play_pause"),
        SystemCommand::MediaNext => media_cmd("next"),
        SystemCommand::MediaPrev => media_cmd("prev"),
        SystemCommand::None => String::new(),
    }
}

fn launch_app(state: &AppState, name: &str) -> String {
    let map = crate::voice::apps::app_map(&state.data_dir);
    let Some(cmd) = crate::voice::apps::resolve_app(&map, name) else {
        return format!("没有找到应用「{}」，可在设置里配置", name);
    };
    crate::voice::launch::launch_app(cmd)
}

fn launch_with(state: &AppState, app_name: &str, target: &str) -> String {
    let map = crate::voice::apps::app_map(&state.data_dir);
    let Some(cmd) = crate::voice::apps::resolve_app(&map, app_name) else {
        return format!("没有找到应用「{}」，可在设置里配置", app_name);
    };
    crate::voice::launch::launch_with(cmd, target)
}

fn play_music(state: &AppState) -> String {
    // 通过音乐播放协议直接唤起播放（orpheus://radio 私人 FM 打开即播），
    // 不再依赖"启动 + 媒体键"（媒体键无法让无曲目的播放器开始播放）
    let cfg = crate::config::load_config(&state.data_dir);
    let source = if cfg.music_play_source.is_empty() {
        "orpheus://radio"
    } else {
        cfg.music_play_source.as_str()
    };
    match opener::open(source) {
        Ok(_) => "好的，已开始播放".into(),
        Err(e) => format!("打开失败：{e}"),
    }
}

fn set_volume_cmd(scale: f32) -> String {
    #[cfg(windows)]
    {
        if scale >= 0.0 && scale <= 1.0 {
            match crate::voice::launch::win::set_volume(scale) {
                Ok(_) => format!("音量已调到 {}", (scale * 100.0).round() as u32),
                Err(e) => format!("音量调节失败：{e}"),
            }
        } else {
            // 增量：读取当前音量 + scale
            match current_volume() {
                Some(cur) => {
                    let target = (cur + scale).clamp(0.0, 1.0);
                    match crate::voice::launch::win::set_volume(target) {
                        Ok(_) => format!("音量已调到 {}", (target * 100.0).round() as u32),
                        Err(e) => format!("音量调节失败：{e}"),
                    }
                }
                None => "音量调节失败：无法读取当前音量".into(),
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = scale;
        "音量调节仅在 Windows 可用".into()
    }
}

#[cfg(windows)]
fn current_volume() -> Option<f32> {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
    const CLSID_MM_DEVICE_ENUMERATOR: windows::core::GUID =
        windows::core::GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&CLSID_MM_DEVICE_ENUMERATOR, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let volume: IAudioEndpointVolume = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()?;
        let level = unsafe { volume.GetMasterVolumeLevelScalar().ok()? };
        Some(level)
    }
}

fn set_mute_cmd(mute: bool) -> String {
    #[cfg(windows)]
    {
        match crate::voice::launch::win::set_mute(mute) {
            Ok(_) => if mute { "已静音".into() } else { "已恢复声音".into() },
            Err(e) => format!("静音操作失败：{e}"),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = mute;
        "静音仅在 Windows 可用".into()
    }
}

fn media_cmd(action: &str) -> String {
    #[cfg(windows)]
    {
        match action {
            "next" => crate::voice::launch::win::media_next(),
            "prev" => crate::voice::launch::win::media_prev(),
            _ => crate::voice::launch::win::media_play_pause(),
        }
        match action {
            "next" => "已切换下一首".into(),
            "prev" => "已切换上一首".into(),
            _ => "已播放".into(),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = action;
        "媒体控制仅在 Windows 可用".into()
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
