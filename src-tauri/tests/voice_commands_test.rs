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
            | (SystemCommand::None, "none")
    )
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
