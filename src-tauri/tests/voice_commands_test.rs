use smart_bc::voice::commands::{DeviceTarget, SystemCommand, parse_system_command};

fn is(cmd: SystemCommand, kind: &str) -> bool {
    matches!(
        (cmd, kind),
        (SystemCommand::ListHistory, "history")
            | (SystemCommand::ListReminders, "reminders")
            | (SystemCommand::ListPeople, "people")
            | (SystemCommand::Status, "status")
            | (SystemCommand::SwitchDevice(DeviceTarget::Internal), "internal")
            | (SystemCommand::SwitchDevice(DeviceTarget::Headset), "headset")
            | (SystemCommand::LaunchApp(_), "launch")
            | (SystemCommand::LaunchWith(_, _), "launch_with")
            | (SystemCommand::Search(_), "search")
            | (SystemCommand::OpenUrl(_), "open_url")
            | (SystemCommand::Volume(_), "volume")
            | (SystemCommand::Mute(_), "mute")
            | (SystemCommand::PlayMusic, "play_music")
            | (SystemCommand::MediaPlayPause, "media_pp")
            | (SystemCommand::MediaNext, "media_next")
            | (SystemCommand::MediaPrev, "media_prev")
            | (SystemCommand::None, "none")
    )
}

#[test]
fn parses_launch_app() {
    assert!(is(parse_system_command("打开计算器"), "launch"));
    assert!(is(parse_system_command("启动记事本"), "launch"));
    assert!(is(parse_system_command("开启音乐"), "launch"));
    assert!(is(parse_system_command("帮我打开浏览器"), "launch"));
}

#[test]
fn parses_launch_with() {
    let cmd = parse_system_command("用记事本打开 D:\\笔记.txt");
    match cmd {
        SystemCommand::LaunchWith(app, target) => {
            assert_eq!(app, "记事本");
            assert_eq!(target, "D:\\笔记.txt");
        }
        _ => panic!("应解析为 LaunchWith，got {cmd:?}"),
    }
}

#[test]
fn parses_search() {
    let cmd = parse_system_command("搜索今天天气");
    match cmd {
        SystemCommand::Search(q) => assert_eq!(q, "今天天气"),
        _ => panic!("应解析为 Search，got {cmd:?}"),
    }
}

#[test]
fn parses_volume_and_mute() {
    assert!(is(parse_system_command("调高音量"), "volume"));
    assert!(is(parse_system_command("调低音量"), "volume"));
    assert!(is(parse_system_command("音量调到50"), "volume"));
    assert!(is(parse_system_command("静音"), "mute"));
}

#[test]
fn parses_media_controls() {
    assert!(is(parse_system_command("播放"), "media_pp"));
    assert!(is(parse_system_command("暂停"), "media_pp"));
    assert!(is(parse_system_command("下一首"), "media_next"));
    assert!(is(parse_system_command("上一首"), "media_prev"));
}

#[test]
fn parses_play_music_phrases() {
    assert!(is(parse_system_command("来点音乐"), "play_music"));
    assert!(is(parse_system_command("放首歌"), "play_music"));
    assert!(is(parse_system_command("放音乐"), "play_music"));
    assert!(is(parse_system_command("放点音乐"), "play_music"));
    assert!(is(parse_system_command("听歌"), "play_music"));
    assert!(is(parse_system_command("播放音乐"), "play_music"));
    // 偏好表达不误触
    assert!(is(parse_system_command("我喜欢听音乐"), "none"));
}

#[test]
fn parses_toggle_only_media() {
    assert!(is(parse_system_command("播放"), "media_pp"));
    assert!(is(parse_system_command("暂停"), "media_pp"));
    assert!(is(parse_system_command("继续"), "media_pp"));
    assert!(is(parse_system_command("继续播放"), "media_pp"));
}

#[test]
fn parses_open_url() {
    let cmd = parse_system_command("打开 bing.com");
    match cmd {
        SystemCommand::OpenUrl(url) => assert_eq!(url, "bing.com"),
        _ => panic!("应解析为 OpenUrl，got {cmd:?}"),
    }
}

#[test]
fn validate_url_http_only() {
    use smart_bc::voice::launch::validate_url;
    assert_eq!(validate_url("bing.com").unwrap(), "https://bing.com");
    assert_eq!(validate_url("https://example.com/x").unwrap(), "https://example.com/x");
    assert!(validate_url("file:///c:/x").is_err());
    assert!(validate_url("cmd /c dir").is_err());
    assert!(validate_url("").is_err());
}

#[test]
fn parses_history_commands() {
    assert!(is(parse_system_command("查看历史"), "history"));
    assert!(is(parse_system_command("最近记录"), "history"));
    assert!(is(parse_system_command("最近说了什么"), "history"));
}

#[test]
fn parses_reminder_commands() {
    assert!(is(parse_system_command("查看承诺"), "reminders"));
    assert!(is(parse_system_command("有什么提醒"), "reminders"));
    assert!(is(parse_system_command("查看提醒列表"), "reminders"));
}

#[test]
fn parses_people_and_status_commands() {
    assert!(is(parse_system_command("查看人脉"), "people"));
    assert!(is(parse_system_command("认识谁"), "people"));
    assert!(is(parse_system_command("助手状态"), "status"));
    assert!(is(parse_system_command("模型状态"), "status"));
}

#[test]
fn parses_device_switch_commands() {
    assert!(is(parse_system_command("用内置麦克风"), "internal"));
    assert!(is(parse_system_command("用耳机"), "headset"));
    assert!(is(parse_system_command("换耳机"), "headset"));
}

#[test]
fn normal_speech_not_command() {
    assert!(is(parse_system_command("三分钟后提醒我喝水"), "none"));
    assert!(is(parse_system_command("今天天气怎么样"), "none"));
    assert!(is(parse_system_command("下午三点和张伟开会"), "none"));
}
