/// 执行引擎：启动应用 / 打开 URL / 搜索 / 音量 / 媒体键。
/// 返回播报文本（成功或失败说明）。
use std::path::Path;

pub fn launch_app(command: &str) -> String {
    if command.starts_with("http://") || command.starts_with("https://") {
        return open_url(command);
    }
    match std::process::Command::new(command).spawn() {
        Ok(_) => "好的，已打开".into(),
        Err(e) => format!("打开失败：{e}"),
    }
}

pub fn launch_with(app_command: &str, target: &str) -> String {
    if !Path::new(target).exists() {
        return format!("找不到文件：{target}");
    }
    match std::process::Command::new(app_command).arg(target).spawn() {
        Ok(_) => "好的，已打开文件".into(),
        Err(e) => format!("打开失败：{e}"),
    }
}

pub fn open_url(url: &str) -> String {
    let Ok(full) = validate_url(url) else {
        return validate_url(url).unwrap_err();
    };
    match opener::open(&full) {
        Ok(_) => "好的，已打开网页".into(),
        Err(e) => format!("打开失败：{e}"),
    }
}

pub fn search(query: &str) -> String {
    let encoded = urlencode(query);
    let url = format!("https://www.bing.com/search?q={encoded}");
    open_url(&url)
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 校验 URL（仅 http/https 且含主机），返回补全协议的 URL
pub fn validate_url(input: &str) -> Result<String, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("地址格式不正确".into());
    }
    let with_proto = if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        format!("https://{s}")
    };
    if !(with_proto.starts_with("http://") || with_proto.starts_with("https://")) {
        return Err("不支持打开这个地址".into());
    }
    let after_scheme = with_proto.split_once("://").map(|(_, r)| r).unwrap_or("");
    if after_scheme.is_empty() || !after_scheme.contains('.') {
        return Err("不支持打开这个地址".into());
    }
    Ok(with_proto)
}

#[cfg(windows)]
pub mod win {
    use windows::core::Interface;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
        VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_VOLUME_DOWN,
        VK_VOLUME_UP, VK_VOLUME_MUTE,
    };

    /// 主音量调节（Core Audio）：scale 0.0-1.0
    pub fn set_volume(scale: f32) -> Result<(), String> {
        use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator};
        use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
        use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
        const CLSID_MM_DEVICE_ENUMERATOR: windows::core::GUID =
            windows::core::GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&CLSID_MM_DEVICE_ENUMERATOR, None, CLSCTX_ALL)
                    .map_err(|e| format!("创建设备枚举器失败: {e}"))?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| format!("获取音频设备失败: {e}"))?;
            let volume: IAudioEndpointVolume = device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|e| format!("激活音量接口失败: {e}"))?;
            volume
                .SetMasterVolumeLevelScalar(scale.clamp(0.0, 1.0), std::ptr::null())
                .map_err(|e| format!("设置音量失败: {e}"))?;
        }
        Ok(())
    }

    pub fn set_mute(mute: bool) -> Result<(), String> {
        use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator};
        use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
        use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
        const CLSID_MM_DEVICE_ENUMERATOR: windows::core::GUID =
            windows::core::GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&CLSID_MM_DEVICE_ENUMERATOR, None, CLSCTX_ALL)
                    .map_err(|e| format!("创建设备枚举器失败: {e}"))?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| format!("获取音频设备失败: {e}"))?;
            let volume: IAudioEndpointVolume = device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|e| format!("激活音量接口失败: {e}"))?;
            volume.SetMute(mute.into(), std::ptr::null()).map_err(|e| format!("静音失败: {e}"))?;
        }
        Ok(())
    }

    fn send_media_key(vk: u16) {
        unsafe {
            let input = INPUT {
                r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: KEYEVENTF_SCANCODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn media_play_pause() {
        send_media_key(VK_MEDIA_PLAY_PAUSE.0 as u16);
    }

    pub fn media_next() {
        send_media_key(VK_MEDIA_NEXT_TRACK.0 as u16);
    }

    pub fn media_prev() {
        send_media_key(VK_MEDIA_PREV_TRACK.0 as u16);
    }

    #[allow(dead_code)]
    fn volume_key(up: bool) {
        let vk = if up { VK_VOLUME_UP.0 } else { VK_VOLUME_DOWN.0 } as u16;
        unsafe {
            let input = INPUT {
                r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk),
                        wScan: 0,
                        dwFlags: KEYEVENTF_SCANCODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
}
