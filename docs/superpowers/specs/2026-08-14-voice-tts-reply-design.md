# SmartBC：助理回复语音播报（系统 TTS）— 设计文档

- 日期：2026-08-14
- 状态：草稿（待用户审阅）
- 定位：将助理回复从"仅系统通知"升级为"可配置的通知/语音/两者"，采用 Windows 系统 TTS

## 1. 需求

- 方案：**A. Windows 系统 TTS**（`tts` crate → WinRT SpeechSynthesizer，原生 OneCore 语音）
- 播报方式：**语音 + 通知并存**（可配置），播报失败/听不到时通知兜底
- 设置页可选：仅通知 / 仅语音 / 两者

## 2. 方案确认（探索结论）

- `tts = "0.26.3"`：Windows 默认 **winrt backend**（`windows::Media::SpeechSynthesis::SpeechSynthesizer`），
  无需额外 feature；中文语音取决于系统安装的 zh-CN 语音包（Win10/11 中文系统自带）
- 播报为**同步播放**（几秒音频）→ 必须**异步线程**播报，避免阻塞 run_listener 监听循环

## 3. 设计

### 3.1 配置（config.rs）
```rust
pub reply_mode: String,  // "notification" | "voice" | "both"，默认 "notification"（保持现状）
```

### 3.2 voice/tts.rs — 异步播报
```rust
pub fn speak_async(text: String) {
    std::thread::spawn(move || {
        if let Ok(mut tts) = tts::Tts::new() {
            let _ = tts.speak(text, true);
        }
    });
}
```
- `Tts` 含 `Rc<RwLock>`（非 Send）→ 在线程内创建，避免跨线程传递
- 播报不阻塞监听循环

### 3.3 voice/reply.rs — 回复路由（纯逻辑可测）
```rust
pub enum DeliverAction { Notification, Voice, Both }

pub fn deliver_plan(reply_mode: &str) -> DeliverAction {
    match reply_mode {
        "voice" => DeliverAction::Voice,
        "both" => DeliverAction::Both,
        _ => DeliverAction::Notification,
    }
}
```
dialog.rs 收到回复后按 plan 执行：Notification → 系统通知；Voice → speak_async；Both → 两者。

### 3.4 dialog.rs 接入
- 回复类通知（"在呢请说"、已记录、回答）改走 `deliver_reply`（按 reply_mode）
- 错误类通知保留系统通知（不播报错误）
- `deliver_reply(app, data_dir, message)` 内部按 plan 路由

## 4. 测试

- `deliver_plan` 纯函数：notification/voice/both/未知值
- speak_async 无法单测（硬件），只测路由
- 回归：全量 cargo test

## 5. 边界

- 系统无中文语音包 → tts::Tts::new 失败 → 静默降级（通知已兜底）
- 播报与监听并存：播报线程独立，监听继续；播报声可能被麦克风拾入（VAD 阈值过滤，MVP 可接受）
- 多回复同时播报：speak_async 每次新建 Tts，后播打断前播（MVP 可接受）

## 6. 验证

- cargo test 全量
- 实测：设置"两者" → 唤醒问答 → 听到语音 + 看到通知
