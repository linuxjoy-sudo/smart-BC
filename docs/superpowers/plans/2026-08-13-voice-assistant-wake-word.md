# 语音助手唤醒式连续问答 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将手动录音升级为"唤醒词 → 30s 聆听窗口内连续问答 → 桌面通知回复"的语音助手交互。

**Architecture:** 新增 `src-tauri/src/voice/` 模块（listener/vad/wake/dialog 四单元）。cpal 常驻采集 → 环形缓冲 → 能量 VAD 断句 → whisper 内存转写（唤醒检测 + 整句）→ 复用现有 RAG 问答（`query::search_context` + `answer::answer_question`）→ 桌面通知。状态机 Idle/Active/Processing 驱动工作线程。

**Tech Stack:** Rust (Tauri 2, cpal 0.18, whisper-rs 0.16), React 19, 已有 `tauri_plugin_notification`。

## Global Constraints

- 测试框架：Rust `cargo test`（tests/ 目录集成测试）；AGENTS.md 要求业务函数变更必须同步单元测试、API 变更必须集成测试、行覆盖 ≥ 80%
- 禁止 `as any`/`@ts-ignore`；Rust clippy 零警告（项目惯例）
- `whisper-rs = "0.16.0"`、`cpal = "0.18.1"`，不新增第三方依赖（唤醒检测复用现有 ggml-small）
- 配置默认值：`voice_assistant_enabled=false`、`wake_word="小贝小贝"`、`listen_window_secs=30`、`wake_model=""`
- 所有命令注册到 `src-tauri/src/lib.rs` 的 `invoke_handler`
- 每次提交前运行 `cargo test` 与 `cargo clippy --all-targets`，前端改动运行 `npx tsc --noEmit`

---

### Task 1: config.rs 扩展语音助手字段

**Files:**
- Modify: `src-tauri/src/config.rs:4-8`（Config 结构体）
- Test: `src-tauri/tests/settings_test.rs`（更新现有测试 + 新增）

**Interfaces:**
- Produces: `Config` 新增字段 `voice_assistant_enabled: bool`、`wake_word: String`、`listen_window_secs: u32`、`wake_model: String`，全部 `#[serde(default)]`

- [ ] **Step 1: 更新测试（先写断言）**

在 `tests/settings_test.rs` 的 `config_roundtrip` 中补充字段断言，并新增默认值测试：

```rust
#[test]
fn voice_assistant_config_defaults() {
    let dir = std::env::temp_dir().join("smartbc_voice_cfg_default");
    let cfg = config::load_config(&dir);
    assert!(!cfg.voice_assistant_enabled);
    assert_eq!(cfg.wake_word, "小贝小贝");
    assert_eq!(cfg.listen_window_secs, 30);
    assert_eq!(cfg.wake_model, "");
}

#[test]
fn voice_assistant_config_roundtrip() {
    let dir = std::env::temp_dir().join("smartbc_voice_cfg_rt");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config {
        api_key: String::new(),
        input_device: None,
        voice_assistant_enabled: true,
        wake_word: "你好助手".into(),
        listen_window_secs: 45,
        wake_model: "base".into(),
    };
    config::save_config(&dir, &cfg).unwrap();
    let loaded = config::load_config(&dir);
    assert!(loaded.voice_assistant_enabled);
    assert_eq!(loaded.wake_word, "你好助手");
    assert_eq!(loaded.listen_window_secs, 45);
    assert_eq!(loaded.wake_model, "base");
    std::fs::remove_file(config::config_path(&dir)).ok();
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --test settings_test`
Expected: 编译错误 `no field voice_assistant_enabled`（字段不存在）

- [ ] **Step 3: 实现字段**

修改 `src-tauri/src/config.rs`：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub input_device: Option<usize>,
    #[serde(default)]
    pub voice_assistant_enabled: bool,
    #[serde(default = "default_wake_word")]
    pub wake_word: String,
    #[serde(default = "default_listen_window")]
    pub listen_window_secs: u32,
    #[serde(default)]
    pub wake_model: String,
}

fn default_wake_word() -> String { "小贝小贝".into() }
fn default_listen_window() -> u32 { 30 }
```

同时修正 `tests/settings_test.rs` 中 `config_roundtrip` 与 `api_key_update_preserves_input_device` 的 `Config { api_key, input_device }` 初始化（补 `..Default::default()`）。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test --test settings_test`
Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/config.rs src-tauri/tests/settings_test.rs
git commit -m "feat: config 增加语音助手字段（开关/唤醒词/聆听窗口）"
```

---

### Task 2: Transcriber 支持内存采样转写

**Files:**
- Modify: `src-tauri/src/asr/whisper.rs`
- Test: 新增 `src-tauri/tests/whisper_mem_test.rs`（不依赖真实模型，仅验证接口存在与错误路径）

**Interfaces:**
- Consumes: `crate::asr::pcm::to_mono_f16k(rate, &samples) -> Vec<f32>`（已存在）
- Produces: `Transcriber::transcribe_samples(&self, rate: u32, samples: &[f32]) -> Result<String, String>`；`transcribe` 重构为调用它

- [ ] **Step 1: 写失败测试**

```rust
use smart_bc::asr::whisper::Transcriber;

#[test]
fn transcribe_samples_errors_without_model() {
    let t = Transcriber::new(std::path::Path::new("/nonexistent/model.bin"));
    assert!(t.is_err());
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test whisper_mem_test`
Expected: 编译错误 `cannot find method transcribe_samples`（测试里该测试只验证 new 的失败路径，先确认能编译运行）

- [ ] **Step 3: 重构 whisper.rs**

将 `transcribe` 中的转写主体提取为 `transcribe_samples`：

```rust
#[derive(Clone)]
pub struct Transcriber {
    ctx: WhisperContext,
}

impl Transcriber {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("load whisper model: {e}"))?;
        Ok(Self { ctx })
    }

    pub fn transcribe(&self, wav_path: &Path) -> Result<String, String> {
        let (rate, samples) = read_any_wav(wav_path).map_err(|e| e.to_string())?;
        self.transcribe_samples(rate, &samples)
    }

    pub fn transcribe_samples(&self, rate: u32, samples: &[f32]) -> Result<String, String> {
        let mono = to_mono_f16k(rate, samples);
        if mono.is_empty() {
            return Err("音频为空".into());
        }
        let mut state = self.ctx.create_state().map_err(|e| e.to_string())?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("zh"));
        params.set_n_threads(4);
        params.set_translate(false);
        state
            .full(params, &mono)
            .map_err(|e| format!("whisper run: {e}"))?;
        let mut text = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(seg) = state.get_segment(i) {
                text.push_str(seg.to_str().map_err(|e| e.to_string())?);
            }
        }
        Ok(text.trim().to_string())
    }
}
```

- [ ] **Step 4: 运行测试通过 + 回归**

Run: `cd src-tauri && cargo test --test whisper_mem_test && cargo test --test asr_pcm_test`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/asr/whisper.rs src-tauri/tests/whisper_mem_test.rs
git commit -m "feat: Transcriber 支持内存采样转写（唤醒/断句用）"
```

---

### Task 3: voice/vad.rs 能量断句检测

**Files:**
- Create: `src-tauri/src/voice/mod.rs`、`src-tauri/src/voice/vad.rs`
- Test: `src-tauri/tests/vad_test.rs`

**Interfaces:**
- Produces:
  - `pub fn rms(samples: &[f32]) -> f32`
  - `pub struct EnergyVad { pub threshold: f32, pub speaking: bool, frame: Vec<f32>, frame_len: usize }`
  - `impl EnergyVad { pub fn new(threshold: f32, frame_len: usize) -> Self; pub fn feed(&mut self, samples: &[f32]) -> bool }`
    - `feed` 累积采样到 `frame_len` 帧后计算 RMS，返回 `speaking`（>threshold 为 true，否则 false）

- [ ] **Step 1: 写失败测试**

```rust
use smart_bc::voice::vad::{rms, EnergyVad};

#[test]
fn rms_of_silence_is_zero() {
    let samples = vec![0.0f32; 480];
    assert!(rms(&samples) < 0.0001);
}

#[test]
fn rms_of_constant_signal() {
    let samples = vec![0.5f32; 480];
    assert!((rms(&samples) - 0.5).abs() < 0.001);
}

#[test]
fn vad_detects_voice_and_silence() {
    let mut vad = EnergyVad::new(0.02, 480); // 10ms@48k
    let silence = vec![0.0f32; 480];
    let voice = vec![0.5f32; 480];
    assert!(!vad.feed(&silence));
    assert!(vad.feed(&voice));
    assert!(!vad.feed(&silence));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test vad_test`
Expected: 编译错误 `cannot find module voice`（未注册）或 `function not found`

- [ ] **Step 3: 创建 voice 模块**

`src-tauri/src/voice/mod.rs`：

```rust
pub mod vad;
pub mod wake;
```

（wake 在 Task 4 创建，先只声明 vad 会编译失败，故 Task 3 和 Task 4 一起提交模块声明——实际做法：本任务只建 vad.rs，mod.rs 仅声明 `pub mod vad;`，Task 4 再补 wake）

`src-tauri/src/voice/vad.rs`：

```rust
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

pub struct EnergyVad {
    pub threshold: f32,
    pub speaking: bool,
    frame: Vec<f32>,
    frame_len: usize,
}

impl EnergyVad {
    pub fn new(threshold: f32, frame_len: usize) -> Self {
        Self { threshold, speaking: false, frame: Vec::with_capacity(frame_len), frame_len }
    }

    pub fn feed(&mut self, samples: &[f32]) -> bool {
        self.frame.extend_from_slice(samples);
        while self.frame.len() >= self.frame_len {
            let chunk: Vec<f32> = self.frame.drain(..self.frame_len).collect();
            let e = rms(&chunk);
            self.speaking = e > self.threshold;
        }
        self.speaking
    }
}
```

在 `src-tauri/src/lib.rs` 模块声明区（第 12 行 `pub mod timeparse;` 后）加 `pub mod voice;`。

- [ ] **Step 4: 运行测试通过**

Run: `cd src-tauri && cargo test --test vad_test`
Expected: PASS（3 tests）

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/voice/mod.rs src-tauri/src/voice/vad.rs src-tauri/src/lib.rs src-tauri/tests/vad_test.rs
git commit -m "feat: 能量 VAD 断句检测（voice/vad）"
```

---

### Task 4: voice/wake.rs 唤醒词匹配

**Files:**
- Create: `src-tauri/src/voice/wake.rs`
- Modify: `src-tauri/src/voice/mod.rs`（补 `pub mod wake;`）
- Test: `src-tauri/tests/wake_test.rs`

**Interfaces:**
- Produces: `pub fn contains_wake_word(text: &str, wake_word: &str) -> bool`
  - 忽略文本中的空白、全角/半角标点差异，只要 `wake_word` 作为连续子串出现（去掉空白后 contains）

- [ ] **Step 1: 写失败测试**

```rust
use smart_bc::voice::wake::contains_wake_word;

#[test]
fn matches_exact() {
    assert!(contains_wake_word("小贝小贝，明天几点开会", "小贝小贝"));
}

#[test]
fn matches_with_whitespace() {
    assert!(contains_wake_word("小贝 小贝 帮我查日程", "小贝小贝"));
}

#[test]
fn no_match_without_wake() {
    assert!(!contains_wake_word("明天几点开会", "小贝小贝"));
}

#[test]
fn no_match_partial() {
    assert!(!contains_wake_word("小贝，你好", "小贝小贝"));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test wake_test`
Expected: 编译错误 `cannot find function contains_wake_word`

- [ ] **Step 3: 实现**

`src-tauri/src/voice/wake.rs`：

```rust
pub fn contains_wake_word(text: &str, wake_word: &str) -> bool {
    if text.is_empty() || wake_word.is_empty() { return false; }
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let key: String = wake_word.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains(&key)
}
```

`src-tauri/src/voice/mod.rs` 改为：

```rust
pub mod vad;
pub mod wake;
```

- [ ] **Step 4: 运行测试通过**

Run: `cd src-tauri && cargo test --test wake_test`
Expected: PASS（4 tests）

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/voice/wake.rs src-tauri/src/voice/mod.rs src-tauri/tests/wake_test.rs
git commit -m "feat: 唤醒词匹配（voice/wake）"
```

---

### Task 5: voice/listener.rs 环形缓冲 + 常驻采集

**Files:**
- Create: `src-tauri/src/voice/listener.rs`
- Modify: `src-tauri/src/voice/mod.rs`（补 `pub mod listener;`）
- Test: `src-tauri/tests/listener_test.rs`

**Interfaces:**
- Consumes: `crate::audio::recorder::list_input_devices` 无关；cpal 0.18 API（`default_host`、`default_input_device`、`build_input_stream`、`StreamTrait::play`）
- Produces:
  - `pub struct RingBuffer { data: VecDeque<f32>, capacity: usize }`
  - `impl RingBuffer { pub fn new(capacity_secs: u32, sample_rate: u32) -> Self; pub fn push(&mut self, samples: &[f32]); pub fn snapshot(&self) -> Vec<f32>; pub fn clear(&mut self); }`
- `pub struct Listener { stream: cpal::Stream, pub buffer: Arc<Mutex<RingBuffer>>, pub sample_rate: u32 }`
- `impl Listener { pub fn start(device_index: Option<usize>, buffer_secs: u32) -> Result<Self, String>; pub fn stop(&self) -> Result<(), String>; }`

- [ ] **Step 1: 写失败测试（环形缓冲纯逻辑）**

```rust
use smart_bc::voice::listener::RingBuffer;

#[test]
fn ringbuffer_keeps_latest() {
    let mut rb = RingBuffer::new(2, 10); // 2s @ 10Hz = 20 采样
    rb.push(&vec![1.0f32; 10]);
    rb.push(&vec![2.0f32; 10]);
    rb.push(&vec![3.0f32; 10]); // 溢出最早 10 个
    let snap = rb.snapshot();
    assert_eq!(snap.len(), 20);
    assert!(snap.iter().all(|&s| s == 2.0 || s == 3.0));
}

#[test]
fn ringbuffer_clear() {
    let mut rb = RingBuffer::new(1, 10);
    rb.push(&vec![1.0f32; 10]);
    rb.clear();
    assert!(rb.snapshot().is_empty());
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test listener_test`
Expected: 编译错误 `cannot find module listener` 或 `cannot find struct RingBuffer`

- [ ] **Step 3: 实现 listener.rs**

```rust
use crate::audio::wav::AudioError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct RingBuffer {
    data: VecDeque<f32>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity_secs: u32, sample_rate: u32) -> Self {
        Self { data: VecDeque::new(), capacity: (capacity_secs as usize) * sample_rate as usize }
    }

    pub fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            if self.data.len() >= self.capacity {
                self.data.pop_front();
            }
            self.data.push_back(s);
        }
    }

    pub fn snapshot(&self) -> Vec<f32> {
        self.data.iter().copied().collect()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

pub struct Listener {
    stream: cpal::Stream,
    pub buffer: Arc<Mutex<RingBuffer>>,
    pub sample_rate: u32,
}

impl Listener {
    pub fn start(device_index: Option<usize>, buffer_secs: u32) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = match device_index {
            Some(idx) => {
                let devices: Vec<_> = host
                    .input_devices()
                    .map_err(|e| AudioError(format!("list devices: {e}")))?
                    .collect();
                devices
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| "invalid device index".to_string())?
            }
            None => host
                .default_input_device()
                .ok_or_else(|| "no input device".to_string())?,
        };
        let config = device
            .default_input_config()
            .map_err(|e| AudioError(format!("input config: {e}")))?;
        let sample_rate = config.sample_rate();
        let buffer = Arc::new(Mutex::new(RingBuffer::new(buffer_secs, sample_rate)));
        let buf_cb = Arc::clone(&buffer);
        let stream = device
            .build_input_stream(
                config.into(),
                move |data: &[f32], _| {
                    if let Ok(mut b) = buf_cb.lock() {
                        b.push(data);
                    }
                },
                move |err| eprintln!("voice listener stream error: {err}"),
                None,
            )
            .map_err(|e| AudioError(format!("build stream: {e}")))?;
        stream.play().map_err(|e| AudioError(format!("play: {e}")))?;
        Ok(Self { stream, buffer, sample_rate })
    }

    pub fn stop(&self) -> Result<(), String> {
        self.stream.pause().map_err(|e| e.to_string())
    }
}
```

`src-tauri/src/voice/mod.rs` 补 `pub mod listener;`。

- [ ] **Step 4: 运行测试通过**

Run: `cd src-tauri && cargo test --test listener_test`
Expected: PASS（2 tests）

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/voice/listener.rs src-tauri/src/voice/mod.rs src-tauri/tests/listener_test.rs
git commit -m "feat: 常驻采集环形缓冲（voice/listener）"
```

---

### Task 6: voice/dialog.rs 状态机 + 问答核心提取

**Files:**
- Create: `src-tauri/src/voice/dialog.rs`
- Modify: `src-tauri/src/voice/mod.rs`（补 `pub mod dialog;`）
- Modify: `src-tauri/src/commands/query.rs`（提取 `answer_question_core`）
- Test: 新增 `src-tauri/tests/dialog_test.rs`；更新 `src-tauri/tests/query_test.rs`

**Interfaces:**
- Consumes:
  - `crate::voice::wake::contains_wake_word`
  - `crate::asr::whisper::Transcriber::transcribe_samples(rate, samples)`
  - `crate::query::search_context(&conn, &question, 8)`（已存在，返回 `SearchContext { hits, people, prefs }`）
  - `crate::llm::answer::answer_question(provider, q, hits, people, prefs)`
- Produces:
  - `pub enum DialogState { Idle, Active, Processing }`
  - `pub enum DialogEvent { VoiceStart, WakeWordHit, SentenceEnd, WindowTimeout, ProcessedOk, ProcessedErr, MicUnavailable }`
  - `pub fn transition(state: DialogState, event: DialogEvent) -> DialogState`
  - `pub fn answer_question_core(conn: &rusqlite::Connection, llm: &dyn LlmProvider, question: &str) -> Result<String, String>`（位于 commands/query.rs）

- [ ] **Step 1: 先提取 answer_question_core 并写失败测试**

在 `src-tauri/src/commands/query.rs`：

```rust
pub fn answer_question_core(
    conn: &rusqlite::Connection,
    llm: &dyn LlmProvider,
    question: &str,
) -> Result<String, String> {
    let ctx = crate::query::search_context(conn, question, 8)?;
    answer::answer_question(llm, question, &ctx.hits, &ctx.people, &ctx.prefs)
        .map_err(|e| e.to_string())
}
```

改 `query_memories` 命令调用它：

```rust
#[tauri::command]
pub fn query_memories(state: tauri::State<'_, AppState>, question: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let llm = state.llm.lock().unwrap().clone();
    answer_question_core(&conn, llm.as_ref(), &question)
}
```

在 `src-tauri/tests/dialog_test.rs` 写状态机测试（失败测试）：

```rust
use smart_bc::voice::dialog::{transition, DialogEvent, DialogState};

#[test]
fn idle_to_active_on_wake() {
    let s = transition(DialogState::Idle, DialogEvent::WakeWordHit);
    assert!(matches!(s, DialogState::Active));
}

#[test]
fn idle_stays_idle_without_wake() {
    let s = transition(DialogState::Idle, DialogEvent::VoiceStart);
    assert!(matches!(s, DialogState::Idle));
}

#[test]
fn active_to_processing_on_sentence_end() {
    let s = transition(DialogState::Active, DialogEvent::SentenceEnd);
    assert!(matches!(s, DialogState::Processing));
}

#[test]
fn processing_returns_to_active_on_success() {
    let s = transition(DialogState::Processing, DialogEvent::ProcessedOk);
    assert!(matches!(s, DialogState::Active));
}

#[test]
fn processing_returns_to_active_on_error() {
    let s = transition(DialogState::Processing, DialogEvent::ProcessedErr);
    assert!(matches!(s, DialogState::Active));
}

#[test]
fn active_times_out_to_idle() {
    let s = transition(DialogState::Active, DialogEvent::WindowTimeout);
    assert!(matches!(s, DialogState::Idle));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test dialog_test`
Expected: 编译错误 `cannot find module voice::dialog` 或 `function not found`

- [ ] **Step 3: 实现 dialog.rs 状态机**

`src-tauri/src/voice/dialog.rs`：

```rust
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
```

`src-tauri/src/voice/mod.rs` 补 `pub mod dialog;`。

- [ ] **Step 4: 运行测试通过 + 回归 query_test**

Run: `cd src-tauri && cargo test --test dialog_test && cargo test --test query_test`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/voice/dialog.rs src-tauri/src/voice/mod.rs src-tauri/src/commands/query.rs src-tauri/tests/dialog_test.rs
git commit -m "feat: 对话状态机 + 问答核心提取（voice/dialog）"
```

---

### Task 7: 语音助手命令层 + 麦克风互斥 + 注册

**Files:**
- Create: `src-tauri/src/commands/voice.rs`
- Modify: `src-tauri/src/commands/record.rs`（start_recording 检查互斥）
- Modify: `src-tauri/src/commands/mod.rs`（注册 voice 模块）
- Modify: `src-tauri/src/lib.rs`（注册命令）
- Test: `src-tauri/tests/voice_cmd_test.rs`

**Interfaces:**
- Consumes: `DialogState`、`Listener`、`Transcriber`、`config::load_config`
- Produces:
  - `pub fn voice_assistant_enabled(data_dir: &Path) -> bool`（config 快捷读取）
  - `#[tauri::command] pub fn set_voice_assistant(app: tauri::AppHandle, state: State<AppState>, enabled: bool) -> Result<String, String>`
    - 开启：检查 recorder 是否占用 → spawn `crate::voice::dialog::run_listener(app, state.inner().clone())` → 保存 config → 返回"语音助手已开启"
    - 关闭：保存 config（监听线程循环内检测到关闭即退出）→ 返回"语音助手已关闭"
  - `#[tauri::command] pub fn get_voice_status(state: State<AppState>) -> Result<VoiceStatus, String>`，`VoiceStatus { enabled: bool, state: String }`

- [ ] **Step 1: 写失败测试（互斥与 config 逻辑）**

```rust
use smart_bc::commands::voice::voice_assistant_enabled;
use smart_bc::config;

#[test]
fn voice_assistant_disabled_by_default() {
    let dir = std::env::temp_dir().join("smartbc_voice_cmd_def");
    assert!(!voice_assistant_enabled(&dir));
}

#[test]
fn voice_assistant_enabled_after_config() {
    let dir = std::env::temp_dir().join("smartbc_voice_cmd_on");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config { voice_assistant_enabled: true, ..Default::default() };
    config::save_config(&dir, &cfg).unwrap();
    assert!(voice_assistant_enabled(&dir));
    std::fs::remove_file(config::config_path(&dir)).ok();
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test voice_cmd_test`
Expected: 编译错误 `cannot find module commands::voice`

- [ ] **Step 3: 实现 commands/voice.rs**

`src-tauri/src/commands/voice.rs`：

```rust
use crate::app_state::AppState;
use serde::Serialize;
use std::path::Path;

pub fn voice_assistant_enabled(data_dir: &Path) -> bool {
    crate::config::load_config(data_dir).voice_assistant_enabled
}

#[derive(Serialize)]
pub struct VoiceStatus {
    pub enabled: bool,
    pub state: String,
}

#[tauri::command]
pub fn set_voice_assistant(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<String, String> {
    if enabled {
        if state.recorder.lock().unwrap().is_some() {
            return Err("正在录音中，请先停止录音".into());
        }
        let st = state.inner().clone();
        std::thread::spawn(move || crate::voice::dialog::run_listener(app, st));
        let mut cfg = crate::config::load_config(&state.data_dir);
        cfg.voice_assistant_enabled = true;
        crate::config::save_config(&state.data_dir, &cfg)?;
        Ok("语音助手已开启".into())
    } else {
        let mut cfg = crate::config::load_config(&state.data_dir);
        cfg.voice_assistant_enabled = false;
        crate::config::save_config(&state.data_dir, &cfg)?;
        Ok("语音助手已关闭".into())
    }
}

#[tauri::command]
pub fn get_voice_status(state: tauri::State<'_, AppState>) -> Result<VoiceStatus, String> {
    Ok(VoiceStatus {
        enabled: voice_assistant_enabled(&state.data_dir),
        state: "idle".into(),
    })
}
```

- [ ] **Step 4: 实现 record.rs 互斥 + 注册**

`src-tauri/src/commands/record.rs` 的 `start_recording` 开头（"正在录音中"检查后）加：

```rust
use crate::commands::voice::voice_assistant_enabled;
// ...
if voice_assistant_enabled(&state.data_dir) {
    return Err("语音助手监听中，请先在设置中关闭".into());
}
```

`src-tauri/src/commands/mod.rs`（若不存在则创建，内容为现有命令模块声明）追加：

```rust
pub mod voice;
```

`src-tauri/src/lib.rs` 的 `invoke_handler` 追加：

```rust
commands::voice::set_voice_assistant,
commands::voice::get_voice_status,
```

`src-tauri/src/voice/dialog.rs` 追加可编译的 `run_listener` 骨架（Task 8 填充完整逻辑）：

```rust
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
```

- [ ] **Step 5: 运行测试通过**

Run: `cd src-tauri && cargo test --test voice_cmd_test && cargo check`
Expected: PASS + 编译通过

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/commands/voice.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/record.rs src-tauri/src/voice/dialog.rs src-tauri/src/lib.rs src-tauri/tests/voice_cmd_test.rs
git commit -m "feat: 语音助手命令层与麦克风互斥"
```

---

### Task 8: run_listener 完整工作线程（唤醒→断句→问答→通知）

**Files:**
- Modify: `src-tauri/src/voice/dialog.rs`（填充 `run_listener`）
- Test: `src-tauri/tests/dialog_test.rs`（补充状态上报相关测试）

**Interfaces:**
- Consumes: `Listener::start(device_index, buffer_secs)`、`EnergyVad`、`contains_wake_word`、`Transcriber::transcribe_samples`、`answer_question_core`、`transition`、`tauri_plugin_notification::NotificationExt`
- Produces: `run_listener(app: AppHandle, state: AppState)` —— 常驻线程：采集 → 能量 VAD → 语音活动时唤醒转写 → Active 后 VAD 断句 → 整句转写 → 问答 → 通知 → 窗口重置

- [ ] **Step 1: 写失败测试（窗口超时常量 + 辅助函数）**

在 `src-tauri/src/voice/dialog.rs` 增加可测试辅助：

```rust
pub fn window_expired(active_elapsed: std::time::Duration, window: std::time::Duration) -> bool {
    active_elapsed >= window
}

pub fn silence_exceeded(silence_secs: f64, limit_secs: f64) -> bool {
    silence_secs >= limit_secs
}
```

测试：

```rust
#[test]
fn window_expiry_after_30s() {
    assert!(window_expired(std::time::Duration::from_secs(31), std::time::Duration::from_secs(30)));
    assert!(!window_expired(std::time::Duration::from_secs(10), std::time::Duration::from_secs(30)));
}

#[test]
fn silence_limit_for_sentence_end() {
    assert!(silence_exceeded(1.6, 1.5));
    assert!(!silence_exceeded(0.5, 1.5));
}
```

- [ ] **Step 2: 运行确认失败**

Run: `cd src-tauri && cargo test --test dialog_test`
Expected: 编译错误 `cannot find function window_expired`

- [ ] **Step 3: 填充 run_listener 完整逻辑**

替换 `src-tauri/src/voice/dialog.rs` 中 Task 7 创建的 `run_listener` 骨架为完整实现：

```rust
pub fn run_listener(app: tauri::AppHandle, state: crate::app_state::AppState) {
    use crate::asr::whisper::Transcriber;
    use crate::commands::query::answer_question_core;
    use crate::commands::voice::voice_assistant_enabled;
    use crate::config::load_config;
    use crate::voice::listener::Listener;
    use crate::voice::vad::{rms, EnergyVad};
    use crate::voice::wake::contains_wake_word;
    use std::time::{Duration, Instant};
    use tauri_plugin_notification::NotificationExt;

    let cfg = load_config(&state.data_dir);
    let wake_word = cfg.wake_word.clone();
    let window = Duration::from_secs(cfg.listen_window_secs.max(1) as u64);
    let listener = match Listener::start(None, 5) {
        Ok(l) => l,
        Err(e) => {
            let _ = app.notification().builder().title("SmartBC 语音助手").body(format!("启动失败：{e}")).show();
            return;
        }
    };
    let transcriber = state.transcriber.lock().unwrap().clone();
    let transcriber = match transcriber {
        Some(t) => t,
        None => {
            let _ = app.notification().builder().title("SmartBC 语音助手").body("模型未加载，语音助手不可用").show();
            return;
        }
    };
    let mut vad = EnergyVad::new(0.02, (listener.sample_rate / 100).max(1) as usize);
    let sr = listener.sample_rate as usize;
    let mut buf: Vec<f32> = Vec::new();
    let mut state_machine = DialogState::Idle;
    let mut active_start = Instant::now();
    let mut silence_since = Instant::now();

    loop {
        if !voice_assistant_enabled(&state.data_dir) {
            let _ = listener.stop();
            return;
        }
        let snap = listener.buffer.lock().unwrap().snapshot();
        if snap.is_empty() { std::thread::sleep(Duration::from_millis(100)); continue; }
        let speaking = vad.feed(&snap);
        buf.extend_from_slice(&snap);
        listener.buffer.lock().unwrap().clear();

        match state_machine {
            DialogState::Idle => {
                if speaking && buf.len() > sr * 3 {
                    let chunk: Vec<f32> = buf.split_off(buf.len().saturating_sub(sr * 3));
                    match transcriber.transcribe_samples(listener.sample_rate, &chunk) {
                        Ok(t) if contains_wake_word(&t, &wake_word) => {
                            state_machine = transition(state_machine, DialogEvent::WakeWordHit);
                            active_start = Instant::now();
                            let _ = app.notification().builder().title("SmartBC").body("在呢，请说").show();
                            buf.clear();
                        }
                        _ => { buf.clear(); }
                    }
                } else if buf.len() > sr * 5 {
                    buf.clear();
                }
            }
            DialogState::Active => {
                if !speaking {
                    if silence_since.elapsed().as_secs_f64() >= 1.5 {
                        let sentence: Vec<f32> = buf.drain(..).collect();
                        if !sentence.is_empty() && rms(&sentence) > 0.01 {
                            state_machine = transition(state_machine, DialogEvent::SentenceEnd);
                            let conn = state.conn.lock().unwrap();
                            let llm = state.llm.lock().unwrap().clone();
                            let result = transcriber
                                .transcribe_samples(listener.sample_rate, &sentence)
                                .and_then(|text| answer_question_core(&conn, llm.as_ref(), &text));
                            match result {
                                Ok(ans) => {
                                    let _ = app.notification().builder().title("SmartBC").body(ans).show();
                                    state_machine = transition(state_machine, DialogEvent::ProcessedOk);
                                }
                                Err(e) => {
                                    let _ = app.notification().builder().title("SmartBC").body(format!("查询失败：{e}")).show();
                                    state_machine = transition(state_machine, DialogEvent::ProcessedErr);
                                }
                            }
                            active_start = Instant::now();
                            silence_since = Instant::now();
                        }
                    }
                } else {
                    silence_since = Instant::now();
                }
                if window_expired(active_start.elapsed(), window) {
                    state_machine = transition(state_machine, DialogEvent::WindowTimeout);
                    buf.clear();
                }
            }
            DialogState::Processing => {}
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
```

`dialog.rs` 顶部补充 `use crate::commands::voice::voice_assistant_enabled;` 或用全路径 `crate::commands::voice::voice_assistant_enabled(&state.data_dir)`。

- [ ] **Step 4: 运行测试通过 + 全量回归**

Run: `cd src-tauri && cargo test --test dialog_test && cargo test`
Expected: 全部 PASS

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/voice/dialog.rs src-tauri/tests/dialog_test.rs
git commit -m "feat: 语音助手工作线程（唤醒→断句→问答→通知）"
```

---

### Task 9: 前端设置页开关

**Files:**
- Modify: `src/api.ts`
- Modify: `src/pages/SettingsPage.tsx`
- Modify: `src/styles.css`

**Interfaces:**
- Consumes: 后端命令 `set_voice_assistant(enabled: bool) -> String`、`get_voice_status() -> { enabled: bool, state: string }`
- Produces: 设置页"语音助手"区块：开关 + 状态 + 唤醒词提示

- [ ] **Step 1: api.ts 增加类型与函数**

```ts
export interface VoiceStatus { enabled: boolean; state: string; }
// api 对象内增加：
setVoiceAssistant: (enabled: boolean) => invoke<string>("set_voice_assistant", { enabled }),
getVoiceStatus: () => invoke<VoiceStatus>("get_voice_status"),
```

`Config` 接口增加 `voice_assistant_enabled: boolean`（可选，兼容旧 config：`voice_assistant_enabled?: boolean`）。

- [ ] **Step 2: SettingsPage.tsx 增加 UI**

在 `model-zone`（语音模型区块）之后、"danger-zone" 之前插入：

```tsx
<div className="model-zone">
  <h3>语音助手</h3>
  <label className="switch-row">
    <input
      type="checkbox"
      checked={voiceOn}
      disabled={voiceBusy}
      onChange={toggleVoice}
    />
    常驻监听（唤醒词"小贝小贝"后连续问答）
  </label>
  <p className="muted">
    {voiceOn ? "监听中：说出唤醒词开始对话" : "已关闭：开启后应用将持续监听麦克风"}
  </p>
</div>
```

state 与逻辑：

```tsx
const [voiceOn, setVoiceOn] = useState(false);
const [voiceBusy, setVoiceBusy] = useState(false);

// useEffect 内：
api.getVoiceStatus().then((s) => setVoiceOn(s.enabled)).catch(() => {});

const toggleVoice = async () => {
  setVoiceBusy(true);
  try {
    const r = await api.setVoiceAssistant(!voiceOn);
    setVoiceOn(!voiceOn);
    setMsg(r);
  } catch (e) {
    setMsg(String(e));
  } finally {
    setVoiceBusy(false);
  }
};
```

`styles.css` 增加 `.settings-page .switch-row { display: flex; align-items: center; gap: 8px; margin-top: 6px; }`。

- [ ] **Step 3: 验证前端**

Run: `cd /mnt/d/src/smart-BC && npx tsc --noEmit && npm run build`
Expected: 无错误，构建成功

- [ ] **Step 4: 提交**

```bash
git add src/api.ts src/pages/SettingsPage.tsx src/styles.css
git commit -m "feat: 设置页语音助手开关"
```

---

### Task 10: 全量验证与清理

**Files:** 无新增（仅验证）

- [ ] **Step 1: Rust 全量测试 + clippy**

Run: `cd src-tauri && cargo test && cargo clippy --all-targets`
Expected: 全部测试 PASS，clippy 零警告

- [ ] **Step 2: 前端验证**

Run: `cd /mnt/d/src/smart-BC && npx tsc --noEmit && npm run build`
Expected: 无错误

- [ ] **Step 3: 确认测试覆盖新增模块**

Run: `cd src-tauri && cargo test --test vad_test --test wake_test --test listener_test --test dialog_test --test voice_cmd_test`
Expected: 全部 PASS（覆盖 voice 模块核心逻辑）

- [ ] **Step 4: 清理临时文件**

检查 `git status`，确认无 `build-win.log`、`build-win.bat` 等临时文件残留。

- [ ] **Step 5: 最终提交（如有遗留）**

```bash
git status
git log --oneline -12
```
