# SmartBC 智能语音助手

基于 **Tauri 2 + Rust + React** 的桌面语音助手，面向 Windows。核心交互以**语音为主**：唤醒词触发、语音记录、语音问答、语音设置提醒，辅以桌面通知与简洁界面。

## 功能特性

### 🎙️ 语音助手（核心）
- **唤醒词**："小贝小贝"（可在配置中修改）。唤醒后 30 秒聆听窗口，无需反复唤醒
- **断句识别**：能量 VAD（10ms 帧）检测语音起止，静音 1.5s 自动断句
- **语音记录**：说完内容自动转写 → LLM 抽取记忆（人脉/偏好/事件/提醒）入库 → 语音播报确认
- **语音问答**：无记忆内容时自动切换为问答，查询历史沉淀
- **重复唤醒**：聆听窗口中再说"小贝小贝"可重置窗口
- **回声抑制**：TTS 播报期间暂停麦克风处理，避免播报声被误识别

### ⏰ 定时提醒
- **自然语言时间**：支持"3分钟后"、"明天早上9点"、"周五晚上"、"半小时后"等相对/绝对表达（中英文数字均可）
- **语音补时间**：说"提醒我喝水"（无时间）→ 追问"什么时候提醒你喝水？" → 回答"下午三点"即设置
- **到点播报**：助理口吻播报"到时间了，该喝水了" + 桌面通知
- **语音完成/延后**：提醒触发后进入聆听窗口，说"完成"或"延后5分钟"即可处理
- **自动流转**：已通知超 1 天未完成自动标记 expired

### 🧠 记忆沉淀
- 从语音转写中自动抽取：**人脉**（姓名/关系）、**偏好**、**事件摘要**，存入 SQLite
- FTS5 全文检索，支持语音查询历史记忆

### 🎤 录音转写
- 手动录音 → whisper 转写（支持设备选择、音量归一化、简繁转换）

### 🔍 问答
- 基于历史记忆 + LLM 的回答，可查询"上次和张伟聊了什么"类问题

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2 |
| 后端 | Rust（whisper-rs / cpal / rusqlite / tauri-plugin-notification） |
| 前端 | React + TypeScript + Vite |
| 语音识别 | whisper.cpp（ggml 模型，beam search 5，16kHz 单声道） |
| 语音合成 | Windows WinRT TTS（Microsoft Huihui） |
| 数据库 | SQLite（conversations / people / preferences / episodes / reminders / usage_events，FTS5 检索） |
| LLM | 兼容 OpenAI 接口的任意 provider（API Key 配置） |

## 快速开始

### 环境要求
- Windows 10/11（TTS 与通知依赖 Windows 平台能力）
- Rust 工具链（MSVC）、Node.js 18+
- 模型文件（whisper ggml 格式）：
  - `ggml-base.bin` — 唤醒词快速检测
  - `ggml-small.bin` — 内容转写（更高精度）

### 安装与运行

```bash
# 安装前端依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布版
npm run tauri build -- --no-bundle
```

### 首次配置
1. **模型**：将 `ggml-base.bin` / `ggml-small.bin` 放入 `C:\Users\<用户>\AppData\Local\smartbc\models\`（或应用数据目录对应位置）
2. **LLM API Key**：应用设置页填写（OpenAI 兼容接口）
3. **麦克风权限**：确认 Windows 隐私设置已允许应用访问麦克风

## 使用指南（语音交互）

### 唤醒与记录
```
你："小贝小贝"
助手："在呢，请说"
你："下午三点和张伟开会"
助手："已记录：人脉：张伟（同事）；事件：下午三点和张伟开会"
```

### 设置提醒
```
你："小贝小贝，三分钟后提醒我喝水"
助手："已记录：提醒：喝水"（3 分钟后）
助手："到时间了，该喝水了"
你："延后5分钟" / "完成了"
助手："好的，延后到 14:35" / "好的，已帮你完成"
```

### 无时间提醒（语音补时间）
```
你："提醒我喝水"
助手："好的，什么时候提醒你喝水？"
你："下午三点"
助手："好的，今天15:00提醒你喝水"
```

### 语音问答
```
你："上次和张伟聊了什么？"
助手：（基于记忆库回答）
```

### 录音转写
界面「录音」页：选择设备 → 开始录音 → 停止 → 自动转写并保存历史。

## 配置（config.json）

位于应用数据目录（`C:\Users\<用户>\AppData\Local\smartbc\config.json`）：

| 字段 | 说明 | 默认 |
|---|---|---|
| `api_key` | LLM API Key | 空 |
| `input_device` | 录音输入设备索引（null=系统默认） | null |
| `voice_assistant_enabled` | 语音助手总开关 | true |
| `wake_word` | 唤醒词 | "小贝小贝" |
| `listen_window_secs` | 聆听窗口时长（秒） | 30 |
| `wake_model` | 唤醒模型（base/small） | base |
| `reply_mode` | 播报方式：`notification` / `voice` / `both` | both |

## 项目结构

```
src/                    前端（React + TS）
  pages/               RecordPage 录音 / QueryPage 问答 / HistoryPage 历史
                       PeoplePage 人脉 / RemindersPage 承诺 / SettingsPage 设置
src-tauri/src/         后端（Rust）
  voice/               语音链路
    dialog.rs          状态机：唤醒→聆听→断句→转写→记录/问答/补时间/提醒响应
    listener.rs        cpal 音频采集（立体声去交错）
    vad.rs             能量 VAD（去抖/迟滞）
    wake.rs            唤醒词匹配（拼音 + 送气声母容错 + 模糊匹配）
    tts.rs             Windows TTS 播报（含回声抑制标志）
    reply.rs           播报分发（通知/语音/both）
  asr/                 whisper 转写（beam search、归一化、重采样）
  memory/              LLM 抽取（人脉/偏好/事件/提醒）+ 内容清洗
  timeparse.rs         中文自然语言时间解析
  scheduler.rs         提醒调度（30s 扫描 + tokio 精确定时器）
  db/                  SQLite（schema 迁移、提醒、记忆、检索）
  commands/            Tauri 命令层
```

## 测试

```bash
cd src-tauri
cargo test              # 全量测试（wake/vad/timeparse/scheduler/reminder/...）
# 针对性测试（快速，推荐开发时用）
cargo test --test wake_test --test timeparse_test
```

测试覆盖：唤醒词匹配（同音/送气/插入字容错）、VAD 状态机、时间解析、提醒调度、数据库迁移、LLM 抽取解析等。

## 常见问题

- **麦克风未拾音**：检查 Windows 隐私设置 → 麦克风权限；在设置页重新选择输入设备（立体声设备已自动去交错处理）
- **唤醒不灵**：确认模型文件存在且为 ggml 格式；环境噪音较大时可换用内置麦克风
- **提醒不触发**：应用需保持运行（本地调度）；确认提醒带时间（无时间提醒会语音追问）
