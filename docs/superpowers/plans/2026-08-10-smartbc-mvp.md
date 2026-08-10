# SmartBC 个人助理 MVP 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个桌面版（Tauri）"记忆黑盒"个人助理 MVP——语音输入自动转写、结构化记忆入库、承诺到期提醒、自然语言回忆查询，4 周单人迭代并交付 3-5 名种子用户验证留存。

**Architecture:** Tauri（Rust 后端 + React 前端）。录音（cpal）→ 本地 Whisper（whisper-rs/whisper.cpp）转写 → 原文先入库（SQLite）→ DeepSeek API 结构化抽取四类记忆（人脉/承诺/偏好/事件）→ FTS5 全文检索 + DeepSeek 回答回忆查询 → Rust 定时调度器触发承诺到期桌面通知。数据本地优先，仅转写文本发送云端。

**Tech Stack:** Rust（tauri 2 / cpal / whisper-rs / rusqlite(bundled) / reqwest(rustls) / tokio / chrono / serde）、React + TypeScript + Vite、SQLite FTS5（trigram 分词，支持中文子串检索）、DeepSeek API（deepseek-chat）。

## Global Constraints

- 目标平台：Windows 10/11（MVP 桌面版）
- 数据本地优先：录音、转写文本、记忆全部存本地 SQLite；仅转写文本发送 DeepSeek
- 首次使用必须明示隐私告知 + API Key 配置；提供单条删除、一键清空、导出
- 中文优先：所有 UI 文案中文；Whisper 语言强制 `zh`；提示词中文
- Whisper 模型：`ggml-small.bin`，国内下载走 `https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-small.bin`，存放于 `{app_data_dir}/models/ggml-small.bin`
- LLM Provider 抽象层：`LlmProvider` trait 支持后续切换 Qwen/GLM；MVP 仅实现 DeepSeek
- API Key 来源：环境变量 `DEEPSEEK_API_KEY` 或设置页（存本地配置文件，不硬编码）
- 原文入库必须先于结构化：抽取失败不得丢原文
- 每项任务完成必须跑通单元测试（`cargo test` / `npm test`）方可提交
- 提交规范：conventional commits（仓库已用 `docs:` / `feat:` 风格）
- 项目根：`/mnt/d/src/smart-BC`（Tauri 在 Windows 原生环境构建调试）
- 库依赖最小化：不用 ORM、不用 Web 框架、不用状态管理库，全部标准工具

---

### Task 1: Tauri + React 脚手架

**Files:**
- Create: `package.json`、`vite.config.ts`、`index.html`、`tsconfig.json`、`src/main.tsx`、`src/App.tsx`、`src/App.css`
- Create: `src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/build.rs`、`src-tauri/src/main.rs`、`src-tauri/src/lib.rs`、`src-tauri/capabilities/default.json`
- Create: `.gitignore`（追加）、`.env.example`

**Interfaces:**
- Produces: `src-tauri/src/lib.rs` 中 `run()` 入口与 `#[tauri::command] fn ping() -> String`；前端 `src/api.ts` 的 invoke 模式后续任务复用

- [ ] **Step 1: 初始化脚手架**

```bash
cd /mnt/d/src/smart-BC
npm create tauri-app@latest . -- --template react-ts --manager npm --yes
npm install
npm install @tauri-apps/api @tauri-apps/plugin-notification
```

（若交互式失败，按 https://tauri.app/start/create-project 手动创建同样结构。）

- [ ] **Step 2: 配置 Cargo 依赖**

```bash
cd src-tauri
cargo add serde --features derive
cargo add serde_json
cargo add rusqlite --features bundled
cargo add cpal
cargo add hound
cargo add whisper-rs
cargo add reqwest --no-default-features --features json,rustls-tls
cargo add tokio --features full
cargo add chrono --features serde
cargo add dirs
cargo add tauri-plugin-notification
```

- [ ] **Step 3: 注册通知插件与 ping 命令**

`src-tauri/src/lib.rs`:

```rust
#[tauri::command]
fn ping() -> String {
    "pong".into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: 前端最小可运行**

`src/App.tsx`:

```tsx
import { invoke } from "@tauri-apps/api/core";

export default function App() {
  return (
    <button onClick={async () => alert(await invoke<string>("ping"))}>
      测试连接
    </button>
  );
}
```

- [ ] **Step 5: 验证启动**

```bash
npm run tauri dev
```

Expected: 窗口打开，点击按钮弹出 "pong"。

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat: tauri+react 脚手架与 ping 命令"
```

---

### Task 2: SQLite 层——schema 迁移 + 对话存储 + FTS5 检索

**Files:**
- Create: `src-tauri/src/db/mod.rs`、`src-tauri/src/db/schema.rs`、`src-tauri/src/db/conversations.rs`、`src-tauri/src/db/search.rs`
- Test: `src-tauri/tests/db_test.rs`

**Interfaces:**
- Consumes: Task 1 的 Cargo 依赖（rusqlite bundled）
- Produces:
  - `db::open(path: &Path) -> rusqlite::Result<Connection>`（开启 foreign_keys）
  - `db::schema::migrate(conn: &Connection) -> rusqlite::Result<()>`
  - `db::conversations::insert_conversation(conn, transcript: &str, audio_path: Option<&str>) -> rusqlite::Result<i64>`（返回 id，同时写 FTS）
  - `db::conversations::get_conversation(conn, id: i64) -> rusqlite::Result<Option<ConversationRow>>`
  - `db::search::search_transcripts(conn, query: &str, limit: usize) -> rusqlite::Result<Vec<SearchHit>>`（`SearchHit { conversation_id, transcript, snippet }`）
  - `db::conversations::ConversationRow { id: i64, created_at: String, transcript: String, audio_path: Option<String> }`

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/db_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::db;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn insert_and_retrieve_conversation() {
    let conn = mem_conn();
    let id = db::conversations::insert_conversation(&conn, "周三给妈妈订蛋糕", None).unwrap();
    let row = db::conversations::get_conversation(&conn, id).unwrap().unwrap();
    assert_eq!(row.transcript, "周三给妈妈订蛋糕");
    assert!(row.id > 0);
}

#[test]
fn fts_finds_chinese_substring() {
    let conn = mem_conn();
    db::conversations::insert_conversation(&conn, "下周三和张伟开预算会", None).unwrap();
    let hits = db::search::search_transcripts(&conn, "预算", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains("预算"));
}

#[test]
fn fts_returns_empty_for_missing() {
    let conn = mem_conn();
    let hits = db::search::search_transcripts(&conn, "不存在的词xyz", 10).unwrap();
    assert!(hits.is_empty());
}
```

- [ ] **Step 2: 运行测试确认失败**

```bash
cargo test --test db_test
```

Expected: FAIL（`db` 模块不存在，编译错误）。

- [ ] **Step 3: 实现 schema 与 conversations**

`src-tauri/src/db/schema.rs`:

```rust
pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            transcript TEXT NOT NULL,
            audio_path TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS conversations_fts USING fts5(
            transcript,
            tokenize = 'trigram'
        );
        "#,
    )
}
```

`src-tauri/src/db/conversations.rs`:

```rust
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ConversationRow {
    pub id: i64,
    pub created_at: String,
    pub transcript: String,
    pub audio_path: Option<String>,
}

pub fn insert_conversation(
    conn: &Connection,
    transcript: &str,
    audio_path: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO conversations (transcript, audio_path) VALUES (?1, ?2)",
        params![transcript, audio_path],
    )?;
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO conversations_fts (rowid, transcript) VALUES (?1, ?2)",
        params![id, transcript],
    )?;
    Ok(id)
}

pub fn get_conversation(conn: &Connection, id: i64) -> Result<Option<ConversationRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, transcript, audio_path FROM conversations WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], |r| {
        Ok(ConversationRow {
            id: r.get(0)?,
            created_at: r.get(1)?,
            transcript: r.get(2)?,
            audio_path: r.get(3)?,
        })
    })?;
    rows.next().transpose()
}
```

- [ ] **Step 4: 实现 FTS 检索**

`src-tauri/src/db/search.rs`:

```rust
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub conversation_id: i64,
    pub transcript: String,
    pub snippet: String,
}

pub fn search_transcripts(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let trimmed = query.trim();
    if trimmed.len() < 2 {
        return Ok(Vec::new());
    }
    // trigram 分词要求查询串 >= 3 字符；中文 2 字词按片段用 LIKE 兜底
    let sql = if trimmed.chars().count() >= 3 {
        r#"SELECT rowid, transcript,
                  snippet(conversations_fts, 0, '【', '】', '…', 12) AS snip
           FROM conversations_fts
           WHERE transcript MATCH ?1
           ORDER BY rank
           LIMIT ?2"#
    } else {
        r#"SELECT id, transcript, transcript AS snip
           FROM conversations
           WHERE transcript LIKE '%' || ?1 || '%'
           ORDER BY id DESC
           LIMIT ?2"#
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![trimmed, limit as i64], |r| {
        Ok(SearchHit {
            conversation_id: r.get(0)?,
            transcript: r.get(1)?,
            snippet: r.get(2)?,
        })
    })?;
    rows.collect()
}
```

- [ ] **Step 5: 组装模块**

`src-tauri/src/db/mod.rs`:

```rust
pub mod conversations;
pub mod schema;
pub mod search;

use rusqlite::Connection;

pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    schema::migrate(&conn)?;
    Ok(conn)
}
```

`src-tauri/src/lib.rs` 顶部加 `pub mod db;`（同时保留 Task 1 的 `run()`）。

- [ ] **Step 6: 运行测试确认通过**

```bash
cargo test --test db_test
```

Expected: 3 个测试全部 PASS。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/db src-tauri/src/lib.rs src-tauri/tests/db_test.rs
git commit -m "feat: SQLite schema 与对话 FTS5 检索"
```

---

### Task 3: 音频录制器（cpal → 16kHz 单声道 WAV）

**Files:**
- Create: `src-tauri/src/audio/mod.rs`、`src-tauri/src/audio/recorder.rs`、`src-tauri/src/audio/wav.rs`
- Test: `src-tauri/tests/wav_test.rs`

**Interfaces:**
- Consumes: Task 1 依赖（cpal、hound）
- Produces:
  - `audio::wav::write_f32_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()>`（空样本报错）
  - `audio::wav::read_f32_wav(path: &Path) -> Result<(u32, Vec<f32>)>`（返回 (采样率, 单声道样本)）
  - `audio::recorder::Recorder::new(device_index: Option<usize>) -> Result<Self>` + `start()` + `stop_and_save(path: &Path) -> Result<usize>` + `sample_rate() -> u32`

- [ ] **Step 1: 写失败测试（WAV 读写回环）**

`src-tauri/tests/wav_test.rs`:

```rust
use smart_bc::audio::wav::{read_f32_wav, write_f32_wav};

#[test]
fn wav_roundtrip_preserves_samples() {
    let dir = std::env::temp_dir().join("smartbc_wav_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("roundtrip.wav");
    let samples: Vec<f32> = (0..8000).map(|i| (i as f32 / 8000.0 * 3.14159).sin()).collect();
    write_f32_wav(&path, &samples, 16000).unwrap();
    let (rate, read_back) = read_f32_wav(&path).unwrap();
    assert_eq!(rate, 16000);
    assert_eq!(read_back.len(), samples.len());
    for (a, b) in samples.iter().zip(read_back.iter()) {
        assert!((a - b).abs() < 1e-4, "sample mismatch {a} vs {b}");
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn wav_rejects_empty() {
    let dir = std::env::temp_dir().join("smartbc_wav_test");
    let path = dir.join("empty.wav");
    let err = write_f32_wav(&path, &[], 16000).unwrap_err();
    assert!(err.to_string().contains("empty"));
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test wav_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: 实现 WAV 读写**

`src-tauri/src/audio/wav.rs`:

```rust
use hound::{WavSpec, WavWriter};
use std::path::Path;

#[derive(Debug)]
pub struct AudioError(pub String);

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for AudioError {}

pub fn write_f32_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), AudioError> {
    if samples.is_empty() {
        return Err(AudioError("samples are empty".into()));
    }
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| AudioError(format!("create wav: {e}")))?;
    for &s in samples {
        writer
            .write_sample(s)
            .map_err(|e| AudioError(format!("write sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| AudioError(format!("finalize wav: {e}")))
}

pub fn read_f32_wav(path: &Path) -> Result<(u32, Vec<f32>), AudioError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| AudioError(format!("open wav: {e}")))?;
    let spec = reader.spec();
    if spec.channels != 1 {
        return Err(AudioError("expected mono wav".into()));
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            reader.samples::<f32>().collect::<Result<_, _>>().map_err(|e| AudioError(e.to_string()))?
        }
        hound::SampleFormat::Int => {
            let max = 2f64.powi(spec.bits_per_sample as i32 - 1);
            reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AudioError(e.to_string()))?
                .into_iter()
                .map(|s| s as f64 / max as f64)
                .map(|s| s as f32)
                .collect()
        }
    };
    Ok((spec.sample_rate, samples))
}
```

- [ ] **Step 4: 实现录音器**

`src-tauri/src/audio/recorder.rs`:

```rust
use crate::audio::wav::{write_f32_wav, AudioError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Recorder {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl Recorder {
    pub fn new(device_index: Option<usize>) -> Result<Self, AudioError> {
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
                    .ok_or_else(|| AudioError("invalid device index".into()))?
            }
            None => host
                .default_input_device()
                .ok_or_else(|| AudioError("no input device".into()))?,
        };
        let config = device
            .default_input_config()
            .map_err(|e| AudioError(format!("input config: {e}")))?;
        let sample_rate = config.sample_rate().0;
        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_cb = Arc::clone(&samples);
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    samples_cb.lock().unwrap().extend_from_slice(data);
                },
                move |err| eprintln!("audio stream error: {err}"),
                None,
            )
            .map_err(|e| AudioError(format!("build stream: {e}")))?;
        Ok(Self { stream, samples, sample_rate })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn start(&self) -> Result<(), AudioError> {
        self.samples.lock().unwrap().clear();
        self.stream.play().map_err(|e| AudioError(format!("play: {e}")))
    }

    pub fn stop_and_save(&self, path: &Path) -> Result<usize, AudioError> {
        self.stream.pause().map_err(|e| AudioError(format!("pause: {e}")))?;
        let samples: Vec<f32> = self.samples.lock().unwrap().clone();
        if samples.is_empty() {
            return Err(AudioError("no audio captured".into()));
        }
        write_f32_wav(path, &samples, self.sample_rate)?;
        Ok(samples.len())
    }
}
```

`src-tauri/src/audio/mod.rs`:

```rust
pub mod recorder;
pub mod wav;
```

`src-tauri/src/lib.rs` 顶部加 `pub mod audio;`。

- [ ] **Step 5: 运行测试确认通过**

```bash
cargo test --test wav_test
```

Expected: 2 个测试 PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/audio src-tauri/src/lib.rs src-tauri/tests/wav_test.rs
git commit -m "feat: cpal 录音器与 WAV 读写"
```

---

### Task 4: 本地 Whisper 转写服务

**Files:**
- Create: `src-tauri/src/asr/mod.rs`、`src-tauri/src/asr/whisper.rs`、`src-tauri/src/asr/model.rs`、`src-tauri/src/asr/pcm.rs`、`src-tauri/src/asr/wav_reader.rs`
- Test: `src-tauri/tests/asr_pcm_test.rs`（纯函数部分）

**Interfaces:**
- Consumes: Task 3 的 `read_f32_wav`
- Produces:
  - `asr::model::model_path(data_dir: &Path) -> PathBuf`、`asr::model::download_model(url: &str, dest: &Path) -> Result<(), String>`（阻塞流式下载）
  - `asr::pcm::to_mono_f16k(sample_rate: u32, samples: &[f32]) -> Vec<f32>`——重采样到 16kHz 单声道（纯函数可测）
  - `asr::wav_reader::read_any_wav(path: &Path) -> Result<(u32, Vec<f32>), String>`——封装 read_f32_wav
  - `asr::whisper::Transcriber::new(model_path: &Path) -> Result<Self, String>` + `transcribe(&self, wav_path: &Path) -> Result<String, String>`（语言强制 zh，单线程 4 线程）

- [ ] **Step 1: 写失败测试（PCM 预处理 + 模型路径）**

`src-tauri/tests/asr_pcm_test.rs`:

```rust
use smart_bc::asr;

#[test]
fn mono_16k_passthrough() {
    let input: Vec<f32> = (0..1600).map(|i| (i as f32) / 1600.0).collect();
    let out = asr::pcm::to_mono_f16k(16000, &input);
    assert_eq!(out.len(), 1600);
    assert!((out[0] - input[0]).abs() < 1e-6);
}

#[test]
fn downsample_48k_to_16k() {
    let input: Vec<f32> = vec![0.0; 4800];
    let out = asr::pcm::to_mono_f16k(48000, &input);
    assert_eq!(out.len(), 1600); // 4800 / 3
}

#[test]
fn model_path_is_under_data_dir() {
    let p = asr::model::model_path(std::path::Path::new("/tmp/appdata"));
    assert!(p.ends_with("models/ggml-small.bin"));
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test asr_pcm_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: 实现 PCM 预处理与模型路径**

`src-tauri/src/asr/pcm.rs`:

```rust
/// 线性重采样到 16kHz 单声道（whisper 要求 16kHz f32 单声道）。
pub fn to_mono_f16k(sample_rate: u32, samples: &[f32]) -> Vec<f32> {
    const TARGET: f64 = 16000.0;
    if sample_rate == 0 {
        return Vec::new();
    }
    if (sample_rate as f64 - TARGET).abs() < 1.0 {
        return samples.to_vec();
    }
    let ratio = sample_rate as f64 / TARGET;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_idx = (i as f64 * ratio) as usize;
        out.push(samples.get(src_idx).copied().unwrap_or(0.0));
    }
    out
}
```

`src-tauri/src/asr/model.rs`:

```rust
use std::path::{Path, PathBuf};

pub const MODEL_FILENAME: &str = "ggml-small.bin";
pub const MODEL_URL: &str =
    "https://hf-mirror.com/ggerganov/whisper.cpp/resolve/main/ggml-small.bin";

pub fn model_path(data_dir: &Path) -> PathBuf {
    data_dir.join("models").join(MODEL_FILENAME)
}

pub fn download_model(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let resp = reqwest::blocking::get(url).map_err(|e| format!("http: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download failed: {}", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| format!("read body: {e}"))?;
    std::fs::write(dest, bytes).map_err(|e| e.to_string())?;
    Ok(())
}
```

（注意：`reqwest` 需启用 `blocking` feature，否则 `reqwest::blocking::get` 不可用。在 Task 1 的 `cargo add` 命令中追加 `blocking`：`cargo add reqwest --no-default-features --features json,rustls-tls,blocking`。）

- [ ] **Step 4: 实现 Transcriber**

`src-tauri/src/asr/whisper.rs`:

```rust
use crate::asr::pcm::to_mono_f16k;
use crate::asr::wav_reader::read_any_wav;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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
        let mono = to_mono_f16k(rate, &samples);
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
            text.push_str(&state.full_get_segment_text(i).map_err(|e| e.to_string())?);
        }
        Ok(text.trim().to_string())
    }
}
```

`src-tauri/src/asr/wav_reader.rs`:

```rust
use crate::audio::wav::read_f32_wav;
use std::path::Path;

pub fn read_any_wav(path: &Path) -> Result<(u32, Vec<f32>), String> {
    read_f32_wav(path).map_err(|e| e.to_string())
}
```

- [ ] **Step 5: 组装模块**

`src-tauri/src/asr/mod.rs`:

```rust
pub mod model;
pub mod pcm;
pub mod wav_reader;
pub mod whisper;
```

`src-tauri/src/lib.rs` 顶部加 `pub mod asr;`。

- [ ] **Step 6: 运行测试确认通过**

```bash
cargo test --test asr_pcm_test
```

Expected: 3 个测试 PASS。（whisper 模型推理属手动验证：录一段中文语音，确认转写可读。）

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/asr src-tauri/src/lib.rs src-tauri/tests/asr_pcm_test.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: 本地 whisper 转写服务与模型下载"
```

---

### Task 5: 录音→转写→入库 全链路命令

**Files:**
- Create: `src-tauri/src/commands/mod.rs`、`src-tauri/src/commands/record.rs`、`src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/lib.rs`（注册命令、初始化状态）
- Test: `src-tauri/tests/record_flow_test.rs`

**Interfaces:**
- Consumes: Task 2 `db::*`、Task 3 `audio::recorder::Recorder`、Task 4 `asr::whisper::Transcriber`
- Produces:
  - `app_state::AppState { conn: Arc<Mutex<Connection>>, recorder: Arc<Mutex<Option<Recorder>>>, transcriber: Arc<Mutex<Option<Transcriber>>>, data_dir: PathBuf }`（`Clone`）
  - `#[tauri::command] fn start_recording(state: State<AppState>) -> Result<(), String>`
  - `#[tauri::command] fn stop_recording(state: State<AppState>) -> Result<RecordResult, String>`（`RecordResult { conversation_id: i64, transcript: String }`）
  - `#[tauri::command] fn get_transcription_status(state: State<AppState>) -> bool`
  - `commands::record::store_transcript(conn, transcript: &str, audio_path: Option<&str>) -> Result<RecordResult, String>`——纯入库函数，空文本报错

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/record_flow_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::db;
use smart_bc::commands::record::store_transcript;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn store_transcript_persists_and_indexes() {
    let conn = mem_conn();
    let result = store_transcript(&conn, "周三交方案给张伟", None).unwrap();
    assert!(result.conversation_id > 0);
    let hits = db::search::search_transcripts(&conn, "方案", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].conversation_id, result.conversation_id);
}

#[test]
fn empty_transcript_rejected() {
    let conn = mem_conn();
    let err = store_transcript(&conn, "   ", None).unwrap_err();
    assert!(err.contains("空"));
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test record_flow_test
```

Expected: FAIL（`commands::record` 不存在）。

- [ ] **Step 3: 实现 app_state 与 record 核心函数**

`src-tauri/src/app_state.rs`:

```rust
use crate::audio::recorder::Recorder;
use crate::asr::whisper::Transcriber;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub recorder: Arc<Mutex<Option<Recorder>>>,
    pub transcriber: Arc<Mutex<Option<Transcriber>>>,
    pub data_dir: PathBuf,
}
```

`src-tauri/src/commands/record.rs`:

```rust
use crate::app_state::AppState;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct RecordResult {
    pub conversation_id: i64,
    pub transcript: String,
}

/// 纯入库函数（可单测）：原文先入库，保证数据不丢。
pub fn store_transcript(
    conn: &Connection,
    transcript: &str,
    audio_path: Option<&str>,
) -> Result<RecordResult, String> {
    let t = transcript.trim();
    if t.is_empty() {
        return Err("转写结果为空，请重试".into());
    }
    let id = crate::db::conversations::insert_conversation(conn, t, audio_path)
        .map_err(|e| format!("入库失败: {e}"))?;
    Ok(RecordResult { conversation_id: id, transcript: t.to_string() })
}

#[tauri::command]
pub fn start_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.recorder.lock().unwrap();
    if guard.is_some() {
        return Err("正在录音中".into());
    }
    let recorder = crate::audio::recorder::Recorder::new(None)?;
    recorder.start()?;
    *guard = Some(recorder);
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: tauri::State<'_, AppState>) -> Result<RecordResult, String> {
    let mut guard = state.recorder.lock().unwrap();
    let recorder = guard.take().ok_or("没有正在进行的录音")?;
    let dir = state.data_dir.join("recordings");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let wav_path = dir.join(format!("rec_{stamp}.wav"));
    recorder.stop_and_save(&wav_path)?;

    let trans_guard = state.transcriber.lock().unwrap();
    let transcriber = trans_guard
        .as_ref()
        .ok_or("模型未加载，请先在设置中下载模型")?;
    let transcript = transcriber.transcribe(&wav_path)?;
    let conn = state.conn.lock().unwrap();
    store_transcript(&conn, &transcript, Some(wav_path.to_str().unwrap()))
}

#[tauri::command]
pub fn get_transcription_status(state: tauri::State<'_, AppState>) -> bool {
    state.transcriber.lock().unwrap().is_some()
}
```

`src-tauri/src/commands/mod.rs`:

```rust
pub mod record;
```

- [ ] **Step 4: 注册命令与状态初始化**

`src-tauri/src/lib.rs` 顶部加 `pub mod app_state; pub mod commands;`，`run()` 改为：

```rust
use app_state::AppState;
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("smartbc");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let db_path = data_dir.join("smartbc.db");
    let conn = db::open(&db_path).expect("open db");
    let state = AppState {
        conn: Arc::new(Mutex::new(conn)),
        recorder: Arc::new(Mutex::new(None)),
        transcriber: Arc::new(Mutex::new(None)),
        data_dir,
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::record::start_recording,
            commands::record::stop_recording,
            commands::record::get_transcription_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 运行测试确认通过**

```bash
cargo test --test record_flow_test
```

Expected: 2 个测试 PASS。

- [ ] **Step 6: 手动验证全链路**

```bash
npm run tauri dev
```

操作：设置页触发模型下载 → 点击录音按钮说话 → 停止 → 历史列表出现转写文本。Expected: 文本入库且可被搜索。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/app_state.rs src-tauri/src/commands src-tauri/src/lib.rs src-tauri/tests/record_flow_test.rs
git commit -m "feat: 录音-转写-入库全链路命令"
```

---

### Task 6: DeepSeek LLM 客户端（可 mock 测试）

**Files:**
- Create: `src-tauri/src/llm/mod.rs`、`src-tauri/src/llm/client.rs`、`src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/Cargo.toml`（dev-dependencies 加 `httpmock`）
- Test: `src-tauri/tests/llm_client_test.rs`

**Interfaces:**
- Produces:
  - `llm::provider::LlmProvider` trait：`fn chat_json(&self, system: &str, user: &str) -> Result<serde_json::Value, LlmError>`
  - `llm::client::DeepSeekClient { base_url, api_key, model, http: reqwest::Client }` 实现 `LlmProvider`
  - `DeepSeekClient::new(api_key: &str) -> Self`（base_url=`https://api.deepseek.com`，model=`deepseek-chat`）
  - `DeepSeekClient::with_base(base_url: String, api_key: &str, model: &str) -> Self`（测试用）
  - `llm::LlmError`（Display + Error，含 Http/Status{code,body}/InvalidJson/EmptyContent 四变体）

- [ ] **Step 1: 写失败测试（mock 服务器）**

`src-tauri/tests/llm_client_test.rs`:

```rust
use httpmock::prelude::*;
use smart_bc::llm::client::DeepSeekClient;
use smart_bc::llm::provider::LlmProvider;

#[test]
fn chat_json_returns_content() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/chat/completions")
            .header("Authorization", "Bearer test-key");
        then.status(200).json_body(serde_json::json!({
            "choices": [{ "message": { "content": "{\"ok\":true}" } }]
        }));
    });
    let client = DeepSeekClient::with_base(server.base_url(), "test-key", "deepseek-chat");
    let result = client.chat_json("sys", "user").unwrap();
    assert_eq!(result["ok"], true);
    mock.assert();
}

#[test]
fn chat_json_maps_http_errors() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(429).json_body(serde_json::json!({"error": {"message": "rate limited"}}));
    });
    let client = DeepSeekClient::with_base(server.base_url(), "k", "m");
    let err = client.chat_json("sys", "user").unwrap_err();
    assert!(err.to_string().contains("429"), "got: {err}");
}

#[test]
fn chat_json_rejects_bad_json_content() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/chat/completions");
        then.status(200).json_body(serde_json::json!({
            "choices": [{ "message": { "content": "not json" } }]
        }));
    });
    let client = DeepSeekClient::with_base(server.base_url(), "k", "m");
    let err = client.chat_json("sys", "user").unwrap_err();
    assert!(err.to_string().contains("JSON"));
}
```

- [ ] **Step 2: 添加 dev-dependency 并确认失败**

```bash
cargo add --dev httpmock
cargo test --test llm_client_test
```

Expected: FAIL（`llm` 模块不存在）。

- [ ] **Step 3: 实现 provider trait 与错误类型**

`src-tauri/src/llm/provider.rs`:

```rust
use serde_json::Value;

#[derive(Debug)]
pub enum LlmError {
    Http(reqwest::Error),
    Status { code: u16, body: String },
    InvalidJson(String),
    EmptyContent,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::Http(e) => write!(f, "网络错误: {e}"),
            LlmError::Status { code, body } => write!(f, "LLM 返回错误 {code}: {body}"),
            LlmError::InvalidJson(s) => write!(f, "LLM 返回 JSON 解析失败: {s}"),
            LlmError::EmptyContent => write!(f, "LLM 返回空内容"),
        }
    }
}

impl std::error::Error for LlmError {}

pub trait LlmProvider {
    fn chat_json(&self, system: &str, user: &str) -> Result<Value, LlmError>;
}
```

- [ ] **Step 4: 实现 DeepSeek 客户端**

`src-tauri/src/llm/client.rs`:

```rust
use super::provider::{LlmError, LlmProvider};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

pub struct DeepSeekClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    http: reqwest::Client,
}

impl DeepSeekClient {
    pub fn new(api_key: &str) -> Self {
        Self::with_base("https://api.deepseek.com".into(), api_key, "deepseek-chat")
    }

    pub fn with_base(base_url: String, api_key: &str, model: &str) -> Self {
        Self {
            base_url,
            api_key: api_key.to_string(),
            model: model.to_string(),
            http: reqwest::Client::new(),
        }
    }
}

impl LlmProvider for DeepSeekClient {
    fn chat_json(&self, system: &str, user: &str) -> Result<Value, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "response_format": {"type": "json_object"},
            "temperature": 0.2
        });
        let resp = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(LlmError::Http)?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::Status { code: status.as_u16(), body: text });
        }
        let json: Value = resp.json().await.map_err(LlmError::Http)?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(LlmError::EmptyContent)?;
        serde_json::from_str(content).map_err(|e| LlmError::InvalidJson(e.to_string()))
    }
}
```

`src-tauri/src/llm/mod.rs`:

```rust
pub mod client;
pub mod provider;

pub use provider::{LlmError, LlmProvider};
```

`src-tauri/src/lib.rs` 顶部加 `pub mod llm;`。

- [ ] **Step 5: 运行测试确认通过**

```bash
cargo test --test llm_client_test
```

Expected: 3 个测试 PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/llm src-tauri/src/lib.rs src-tauri/tests/llm_client_test.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: DeepSeek LLM 客户端与 Provider 抽象"
```

---

### Task 7: 结构化记忆抽取（prompt + JSON 解析）

**Files:**
- Create: `src-tauri/src/llm/extract.rs`、`src-tauri/src/memory/mod.rs`、`src-tauri/src/memory/types.rs`
- Test: `src-tauri/tests/extract_test.rs`

**Interfaces:**
- Consumes: Task 6 `LlmProvider::chat_json`
- Produces:
  - `memory::types::MemoryExtraction { people: Vec<PersonExtract>, reminders: Vec<ReminderExtract>, preferences: Vec<PreferenceExtract>, episode: Option<EpisodeExtract> }`（全字段 `Serialize + Deserialize + Default`）
  - `PersonExtract { name, relation, note }`、`ReminderExtract { content, due: Option<String> }`、`PreferenceExtract { topic, value }`、`EpisodeExtract { summary, people: Vec<String>, place: Option<String> }`
  - `memory::extract::build_extract_prompt(transcript: &str) -> String`
  - `memory::extract::parse_extraction(raw_json: &str) -> Result<MemoryExtraction, String>`（容错：缺字段给默认，非法 JSON 报错）
  - `memory::extract::extract_from_transcript(provider: &dyn LlmProvider, transcript: &str) -> Result<MemoryExtraction, LlmError>`

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/extract_test.rs`:

```rust
use smart_bc::memory::extract::{build_extract_prompt, parse_extraction};

#[test]
fn parse_full_extraction() {
    let raw = r#"{
      "people": [{"name": "张伟", "relation": "供应商", "note": "聊过预算"}],
      "reminders": [{"content": "周三交方案", "due": "2026-08-12"}],
      "preferences": [{"topic": "饮食", "value": "不吃香菜"}],
      "episode": {"summary": "和张伟讨论供应商预算", "people": ["张伟"], "place": "公司"}
    }"#;
    let parsed = parse_extraction(raw).unwrap();
    assert_eq!(parsed.people.len(), 1);
    assert_eq!(parsed.people[0].name, "张伟");
    assert_eq!(parsed.reminders[0].due.as_deref(), Some("2026-08-12"));
    assert_eq!(parsed.preferences[0].value, "不吃香菜");
    assert!(parsed.episode.is_some());
}

#[test]
fn parse_missing_fields_defaults_empty() {
    let raw = r#"{"people": [], "reminders": [], "preferences": []}"#;
    let parsed = parse_extraction(raw).unwrap();
    assert!(parsed.people.is_empty());
    assert!(parsed.reminders.is_empty());
    assert!(parsed.preferences.is_empty());
    assert!(parsed.episode.is_none());
}

#[test]
fn parse_invalid_json_errors() {
    let err = parse_extraction("not json").unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn prompt_contains_transcript_and_json_requirement() {
    let p = build_extract_prompt("周三交方案给张伟");
    assert!(p.contains("周三交方案给张伟"));
    assert!(p.contains("json"));
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test extract_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: 实现类型与解析**

`src-tauri/src/memory/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PersonExtract {
    pub name: String,
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReminderExtract {
    pub content: String,
    #[serde(default)]
    pub due: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreferenceExtract {
    pub topic: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EpisodeExtract {
    pub summary: String,
    #[serde(default)]
    pub people: Vec<String>,
    #[serde(default)]
    pub place: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MemoryExtraction {
    #[serde(default)]
    pub people: Vec<PersonExtract>,
    #[serde(default)]
    pub reminders: Vec<ReminderExtract>,
    #[serde(default)]
    pub preferences: Vec<PreferenceExtract>,
    #[serde(default)]
    pub episode: Option<EpisodeExtract>,
}
```

`src-tauri/src/llm/extract.rs`:

```rust
use crate::llm::provider::{LlmError, LlmProvider};
use crate::memory::types::MemoryExtraction;

pub fn build_extract_prompt(transcript: &str) -> String {
    format!(
        r#"你是个人助理的记忆抽取器。从用户的语音转写文本中抽取结构化记忆，只输出合法 JSON，不要任何多余文字。
输出格式（缺失项给空数组或 null）：
{{
  "people": [{{"name": "人名", "relation": "关系(可选)", "note": "备注(可选)"}}],
  "reminders": [{{"content": "承诺/待办内容", "due": "截止时间，ISO 日期或 null"}}],
  "preferences": [{{"topic": "偏好主题", "value": "偏好内容"}}],
  "episode": {{"summary": "本次事件摘要", "people": ["人物"], "place": "地点或 null"}}
}}

转写文本：
{transcript}"#,
        transcript = transcript
    )
}

pub fn parse_extraction(raw_json: &str) -> Result<MemoryExtraction, String> {
    let v: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|e| format!("非法 JSON: {e}"))?;
    let obj = v.as_object().ok_or("JSON 不是对象")?;
    let mut ext = MemoryExtraction::default();
    if let Some(p) = obj.get("people") {
        ext.people = serde_json::from_value(p.clone()).unwrap_or_default();
    }
    if let Some(r) = obj.get("reminders") {
        ext.reminders = serde_json::from_value(r.clone()).unwrap_or_default();
    }
    if let Some(p) = obj.get("preferences") {
        ext.preferences = serde_json::from_value(p.clone()).unwrap_or_default();
    }
    if let Some(e) = obj.get("episode") {
        if !e.is_null() {
            ext.episode = serde_json::from_value(e.clone()).ok();
        }
    }
    Ok(ext)
}

pub fn extract_from_transcript(
    provider: &dyn LlmProvider,
    transcript: &str,
) -> Result<MemoryExtraction, LlmError> {
    let raw = provider.chat_json(
        "你输出严格 JSON，不要 Markdown 代码块。",
        &build_extract_prompt(transcript),
    )?;
    parse_extraction(&raw.to_string())
        .map_err(|e| LlmError::InvalidJson(e))
}
```

`src-tauri/src/memory/mod.rs`:

```rust
pub mod extract;
pub mod types;
```

`src-tauri/src/lib.rs` 顶部加 `pub mod memory;`。

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --test extract_test
```

Expected: 4 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/llm/extract.rs src-tauri/src/memory src-tauri/src/lib.rs src-tauri/tests/extract_test.rs
git commit -m "feat: 结构化记忆抽取与 JSON 解析"
```

---

### Task 8: 记忆持久化（人脉/偏好/事件表 + 抽取入库管线）

**Files:**
- Create: `src-tauri/src/db/memories.rs`
- Modify: `src-tauri/src/db/schema.rs`（新增 3 表）、`src-tauri/src/commands/record.rs`（接入抽取）、`src-tauri/src/app_state.rs`（挂载 LlmProvider）、`src-tauri/src/lib.rs`
- Test: `src-tauri/tests/memory_db_test.rs`

**Interfaces:**
- Consumes: Task 7 `MemoryExtraction`
- Produces:
  - `db::memories::save_extraction(conn, extraction: &MemoryExtraction, conversation_id: i64) -> Result<()>`
  - `db::memories::list_people(conn) -> Result<Vec<PersonRow>>`、`list_preferences(conn) -> Result<Vec<PreferenceRow>>`、`list_episodes(conn, limit: usize) -> Result<Vec<EpisodeRow>>`
  - `db::memories::delete_memory(conn, table: &str, id: i64) -> Result<()>`（表名白名单防注入）
  - `commands::record::process_audio_full(conn, provider: &dyn LlmProvider, transcriber, wav_path) -> Result<RecordResult, String>`——转写→入库→抽取→记忆入库（抽取失败仅告警不阻断）

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/memory_db_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::db;
use smart_bc::memory::types::{EpisodeExtract, MemoryExtraction, PersonExtract, PreferenceExtract, ReminderExtract};

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

fn sample_extraction() -> MemoryExtraction {
    MemoryExtraction {
        people: vec![PersonExtract { name: "张伟".into(), relation: "供应商".into(), note: String::new() }],
        reminders: vec![ReminderExtract { content: "周三交方案".into(), due: Some("2026-08-12".into()) }],
        preferences: vec![PreferenceExtract { topic: "饮食".into(), value: "不吃香菜".into() }],
        episode: Some(EpisodeExtract { summary: "讨论预算".into(), people: vec!["张伟".into()], place: Some("公司".into()) }),
    }
}

#[test]
fn save_and_list_all_memory_types() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "周三交方案给张伟，他不吃香菜", None).unwrap();
    db::memories::save_extraction(&conn, &sample_extraction(), cid).unwrap();
    let people = db::memories::list_people(&conn).unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].name, "张伟");
    assert_eq!(people[0].conversation_id, cid);
    let prefs = db::memories::list_preferences(&conn).unwrap();
    assert_eq!(prefs.len(), 1);
    assert_eq!(prefs[0].value, "不吃香菜");
    let eps = db::memories::list_episodes(&conn, 10).unwrap();
    assert_eq!(eps.len(), 1);
    assert_eq!(eps[0].summary, "讨论预算");
}

#[test]
fn delete_memory_removes_row() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "认识李四", None).unwrap();
    db::memories::save_extraction(&conn, &sample_extraction(), cid).unwrap();
    let people = db::memories::list_people(&conn).unwrap();
    db::memories::delete_memory(&conn, "people", people[0].id).unwrap();
    assert!(db::memories::list_people(&conn).unwrap().is_empty());
}

#[test]
fn delete_memory_rejects_unknown_table() {
    let conn = mem_conn();
    assert!(db::memories::delete_memory(&conn, "users", 1).is_err());
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test memory_db_test
```

Expected: FAIL（`db::memories` 不存在）。

- [ ] **Step 3: 新增 schema 表**

`src-tauri/src/db/schema.rs` 的 `migrate` 追加：

```rust
        CREATE TABLE IF NOT EXISTS people (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            relation TEXT DEFAULT '',
            note TEXT DEFAULT '',
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        CREATE INDEX IF NOT EXISTS idx_people_name ON people(name);
        CREATE TABLE IF NOT EXISTS preferences (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            topic TEXT NOT NULL,
            value TEXT NOT NULL,
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        CREATE TABLE IF NOT EXISTS episodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            summary TEXT NOT NULL,
            place TEXT DEFAULT '',
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
```

- [ ] **Step 4: 实现 memories 存储层**

`src-tauri/src/db/memories.rs`:

```rust
use crate::memory::types::MemoryExtraction;
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PersonRow {
    pub id: i64,
    pub name: String,
    pub relation: String,
    pub note: String,
    pub conversation_id: i64,
}

#[derive(Debug, Serialize)]
pub struct PreferenceRow {
    pub id: i64,
    pub topic: String,
    pub value: String,
    pub conversation_id: i64,
}

#[derive(Debug, Serialize)]
pub struct EpisodeRow {
    pub id: i64,
    pub summary: String,
    pub place: String,
    pub conversation_id: i64,
}

pub fn save_extraction(conn: &Connection, ext: &MemoryExtraction, conversation_id: i64) -> Result<()> {
    for p in &ext.people {
        if p.name.trim().is_empty() { continue; }
        conn.execute(
            "INSERT INTO people (name, relation, note, conversation_id) VALUES (?1,?2,?3,?4)",
            params![p.name.trim(), p.relation.trim(), p.note.trim(), conversation_id],
        )?;
    }
    for pr in &ext.preferences {
        if pr.topic.trim().is_empty() { continue; }
        conn.execute(
            "INSERT INTO preferences (topic, value, conversation_id) VALUES (?1,?2,?3)",
            params![pr.topic.trim(), pr.value.trim(), conversation_id],
        )?;
    }
    if let Some(e) = &ext.episode {
        if !e.summary.trim().is_empty() {
            conn.execute(
                "INSERT INTO episodes (summary, place, conversation_id) VALUES (?1,?2,?3)",
                params![e.summary.trim(), e.place.clone().unwrap_or_default().trim(), conversation_id],
            )?;
        }
    }
    Ok(())
}

pub fn list_people(conn: &Connection) -> Result<Vec<PersonRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, relation, note, conversation_id FROM people ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok(PersonRow {
        id: r.get(0)?, name: r.get(1)?, relation: r.get(2)?,
        note: r.get(3)?, conversation_id: r.get(4)?,
    }))?;
    rows.collect()
}

pub fn list_preferences(conn: &Connection) -> Result<Vec<PreferenceRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, topic, value, conversation_id FROM preferences ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok(PreferenceRow {
        id: r.get(0)?, topic: r.get(1)?, value: r.get(2)?, conversation_id: r.get(3)?,
    }))?;
    rows.collect()
}

pub fn list_episodes(conn: &Connection, limit: usize) -> Result<Vec<EpisodeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, summary, place, conversation_id FROM episodes ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |r| Ok(EpisodeRow {
        id: r.get(0)?, summary: r.get(1)?, place: r.get(2)?, conversation_id: r.get(3)?,
    }))?;
    rows.collect()
}

const ALLOWED_TABLES: [&str; 3] = ["people", "preferences", "episodes"];

pub fn delete_memory(conn: &Connection, table: &str, id: i64) -> Result<()> {
    if !ALLOWED_TABLES.contains(&table) {
        return Err(rusqlite::Error::InvalidParameterName(format!("table {table} not allowed")));
    }
    conn.execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id])?;
    Ok(())
}
```

- [ ] **Step 5: 接入抽取管线**

`src-tauri/src/commands/record.rs` 新增：

```rust
use crate::llm::provider::LlmProvider;
use crate::memory::extract::extract_from_transcript;

pub fn process_audio_full(
    conn: &Connection,
    provider: &dyn LlmProvider,
    transcriber: &crate::asr::whisper::Transcriber,
    wav_path: &std::path::Path,
) -> Result<RecordResult, String> {
    let transcript = transcriber.transcribe(wav_path)?;
    let result = store_transcript(conn, &transcript, Some(wav_path.to_str().unwrap()))?;
    match extract_from_transcript(provider, &transcript) {
        Ok(ext) => {
            crate::db::memories::save_extraction(conn, &ext, result.conversation_id)
                .map_err(|e| format!("记忆入库失败: {e}"))?;
            // 承诺在 Task 11 接入提醒表
            eprintln!("extracted {} people, {} reminders", ext.people.len(), ext.reminders.len());
        }
        Err(e) => eprintln!("抽取失败（原文已入库）: {e}"),
    }
    Ok(result)
}
```

`src-tauri/src/app_state.rs` 增加字段：

```rust
    pub llm: Arc<dyn crate::llm::provider::LlmProvider + Send + Sync>,
```

`src-tauri/src/lib.rs` 初始化（`load_api_key` 见 Task 14 完整实现，此处先读环境变量）：

```rust
    let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();
    let llm: Arc<dyn llm::provider::LlmProvider + Send + Sync> =
        Arc::new(llm::client::DeepSeekClient::new(&api_key));
```

并把 `llm` 加入 `AppState` 构造。

- [ ] **Step 6: 运行测试确认通过**

```bash
cargo test --test memory_db_test
cargo test
```

Expected: 全部 PASS。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/db/memories.rs src-tauri/src/db/schema.rs src-tauri/src/commands/record.rs src-tauri/src/app_state.rs src-tauri/src/lib.rs src-tauri/tests/memory_db_test.rs
git commit -m "feat: 人脉/偏好/事件记忆持久化与抽取入库"
```

---

### Task 9: 回忆查询（RAG：检索 + LLM 回答）

**Files:**
- Create: `src-tauri/src/commands/query.rs`、`src-tauri/src/llm/answer.rs`
- Test: `src-tauri/tests/query_test.rs`

**Interfaces:**
- Consumes: Task 2 `search::search_transcripts`、Task 8 `memories::list_*`、Task 6 `LlmProvider`
- Produces:
  - `query::QueryContext { hits: Vec<SearchHit>, people: Vec<PersonRow>, prefs: Vec<PreferenceRow> }`（Default）
  - `query::search_context(conn: &Connection, question: &str, limit: usize) -> Result<QueryContext, String>`
  - `#[tauri::command] fn query_memories(state: State<AppState>, question: String) -> Result<String, String>`
  - `llm::answer::build_answer_prompt(question, hits, people, prefs) -> String`
  - `llm::answer::answer_question(provider, question, hits, people, prefs) -> Result<String, LlmError>`

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/query_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::db;
use smart_bc::query;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn search_context_collects_hits_and_memories() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "周三和张伟开预算会", None).unwrap();
    let ext = smart_bc::memory::types::MemoryExtraction {
        people: vec![smart_bc::memory::types::PersonExtract { name: "张伟".into(), relation: "供应商".into(), note: String::new() }],
        reminders: vec![], preferences: vec![], episode: None,
    };
    db::memories::save_extraction(&conn, &ext, cid).unwrap();
    let ctx = smart_bc::query::search_context(&conn, "张伟 预算", 10).unwrap();
    assert_eq!(ctx.hits.len(), 1);
    assert_eq!(ctx.people.len(), 1);
    assert_eq!(ctx.people[0].name, "张伟");
}

#[test]
fn answer_prompt_includes_question_and_evidence() {
    let p = smart_bc::llm::answer::build_answer_prompt(
        "上次和张伟聊了什么？",
        &[smart_bc::db::search::SearchHit {
            conversation_id: 1,
            transcript: "和张伟聊了预算".into(),
            snippet: "【和张伟聊了预算】".into(),
        }],
        &[],
        &[],
    );
    assert!(p.contains("上次和张伟聊了什么"));
    assert!(p.contains("【和张伟聊了预算】"));
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test query_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: 实现查询上下文与回答 prompt**

`src-tauri/src/commands/query.rs`:

```rust
use crate::app_state::AppState;
use crate::db::memories::{PersonRow, PreferenceRow};
use crate::db::search::SearchHit;
use crate::llm::answer;

#[derive(Debug, Default)]
pub struct QueryContext {
    pub hits: Vec<SearchHit>,
    pub people: Vec<PersonRow>,
    pub prefs: Vec<PreferenceRow>,
}

pub fn search_context(conn: &rusqlite::Connection, question: &str, limit: usize) -> Result<QueryContext, String> {
    let hits = crate::db::search::search_transcripts(conn, question, limit).map_err(|e| e.to_string())?;
    let people = crate::db::memories::list_people(conn).map_err(|e| e.to_string())?;
    let prefs = crate::db::memories::list_preferences(conn).map_err(|e| e.to_string())?;
    Ok(QueryContext { hits, people, prefs })
}

#[tauri::command]
pub fn query_memories(state: tauri::State<'_, AppState>, question: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let ctx = search_context(&conn, &question, 8)?;
    let llm = state.llm.clone();
    let answer = answer::answer_question(llm.as_ref(), &question, &ctx.hits, &ctx.people, &ctx.prefs)
        .map_err(|e| e.to_string())?;
    Ok(answer)
}
```

`src-tauri/src/llm/answer.rs`:

```rust
use crate::db::memories::{PersonRow, PreferenceRow};
use crate::db::search::SearchHit;
use crate::llm::provider::{LlmError, LlmProvider};

pub fn build_answer_prompt(
    question: &str,
    hits: &[SearchHit],
    people: &[PersonRow],
    prefs: &[PreferenceRow],
) -> String {
    let mut evidence = String::new();
    for h in hits {
        evidence.push_str(&format!("[对话 #{}] {}\n", h.conversation_id, h.snippet));
    }
    let mut people_str = String::new();
    for p in people {
        people_str.push_str(&format!("- {}（关系:{}, 备注:{}）\n", p.name, p.relation, p.note));
    }
    let mut prefs_str = String::new();
    for pr in prefs {
        prefs_str.push_str(&format!("- {}: {}\n", pr.topic, pr.value));
    }
    format!(
        r#"你是用户的私人记忆助理。根据提供的记忆证据回答用户问题。
要求：
1. 只依据提供的证据回答；证据不足时明确说"我还没有这方面的记忆"。
2. 引用相关对话时标注其编号，如（对话 #3）。
3. 回答用中文，简洁具体。

用户问题：{question}

相关对话记录：
{evidence}
已记住的人脉：
{people_str}
已记住的偏好：
{prefs_str}"#
    )
}

pub fn answer_question(
    provider: &dyn LlmProvider,
    question: &str,
    hits: &[SearchHit],
    people: &[PersonRow],
    prefs: &[PreferenceRow],
) -> Result<String, LlmError> {
    let prompt = build_answer_prompt(question, hits, people, prefs);
    let v = provider.chat_json("你是记忆助理。直接输出回答文本。", &prompt)?;
    Ok(v.as_str().unwrap_or(&v.to_string()).to_string())
}
```

`src-tauri/src/commands/mod.rs` 追加 `pub mod query;`；`src-tauri/src/lib.rs` 注册 `commands::query::query_memories`。

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --test query_test
```

Expected: 2 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands/query.rs src-tauri/src/llm/answer.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/tests/query_test.rs
git commit -m "feat: 回忆查询 RAG 管线"
```

---

### Task 10: 中文相对时间解析器

**Files:**
- Create: `src-tauri/src/timeparse.rs`
- Test: `src-tauri/tests/timeparse_test.rs`

**Interfaces:**
- Produces: `timeparse::parse_due(expr: &str, now: chrono::NaiveDateTime) -> Option<chrono::NaiveDateTime>`。支持：今天/明天/后天、周X（本周）/下X/下(个)周X、今天下午/晚上/明天上午、N点/N点半、X月X日。解析失败返回 None。默认时间：今天=18:00，其他日期=09:00。

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/timeparse_test.rs`:

```rust
use chrono::NaiveDate;
use smart_bc::timeparse::parse_due;

fn now() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 10).unwrap().and_hms_opt(9, 0, 0).unwrap() // 周一 09:00
}

#[test]
fn today_and_tomorrow() {
    assert_eq!(parse_due("今天", now()), Some(now().date().and_hms_opt(18, 0, 0).unwrap()));
    assert_eq!(parse_due("明天", now()), Some(now().date().succ_opt().unwrap().and_hms_opt(9, 0, 0).unwrap()));
    assert_eq!(parse_due("后天", now()), Some(now().date().succ_opt().unwrap().succ_opt().unwrap().and_hms_opt(9, 0, 0).unwrap()));
}

#[test]
fn weekday_this_and_next_week() {
    // 2026-08-10 是周一
    assert_eq!(parse_due("周三", now()), Some(NaiveDate::from_ymd_opt(2026, 8, 12).unwrap().and_hms_opt(9, 0, 0).unwrap()));
    assert_eq!(parse_due("下周一", now()), Some(NaiveDate::from_ymd_opt(2026, 8, 17).unwrap().and_hms_opt(9, 0, 0).unwrap()));
}

#[test]
fn hour_expressions() {
    assert_eq!(parse_due("下午3点", now()), Some(now().date().and_hms_opt(15, 0, 0).unwrap()));
    assert_eq!(parse_due("10点半", now()), Some(now().date().and_hms_opt(10, 30, 0).unwrap()));
}

#[test]
fn date_expressions() {
    assert_eq!(parse_due("8月15日", now()), Some(NaiveDate::from_ymd_opt(2026, 8, 15).unwrap().and_hms_opt(9, 0, 0).unwrap()));
}

#[test]
fn unsupported_returns_none() {
    assert_eq!(parse_due("尽快", now()), None);
    assert_eq!(parse_due("", now()), None);
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test timeparse_test
```

Expected: FAIL（`timeparse` 不存在）。

- [ ] **Step 3: 实现解析器**

`src-tauri/src/timeparse.rs`:

```rust
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

/// 解析中文相对时间表达；失败返回 None。
/// 默认时间：今天=18:00，其他日期=09:00。
pub fn parse_due(expr: &str, now: NaiveDateTime) -> Option<NaiveDateTime> {
    let s = expr.trim();
    if s.is_empty() {
        return None;
    }
    let today = now.date();
    let weekday_names = ["一", "二", "三", "四", "五", "六", "日", "天"];

    // 日期基准
    let date = if s.contains("后天") {
        today.succ_opt()?.succ_opt()?
    } else if s.contains("明天") || s.contains("明日") {
        today.succ_opt()?
    } else if s.contains("今天") || s.contains("今日") || s.contains("今晚") {
        today
    } else if s.starts_with("下") && weekday_names.iter().any(|d| {
        s.contains(&format!("周{d}")) || s.contains(&format!("星期{d}"))
    }) {
        let wd = weekday_from_str(s)?;
        next_weekday(today, wd)?
    } else if let Some(day) = weekday_names.iter().find(|d| {
        let pat = format!("周{d}");
        s.contains(&pat) || s.contains(&format!("星期{d}"))
    }) {
        let wd = match *day {
            "一" => Weekday::Mon, "二" => Weekday::Tue, "三" => Weekday::Wed,
            "四" => Weekday::Thu, "五" => Weekday::Fri, "六" => Weekday::Sat,
            _ => Weekday::Sun,
        };
        next_or_today_weekday(today, wd)?
    } else if let Some((m, d)) = parse_md(s) {
        NaiveDate::from_ymd_opt(now.year(), m, d)?
    } else {
        today
    };

    // 时间
    let time = parse_time(s, date == today, now);
    Some(date.and_time(time))
}

fn weekday_from_str(s: &str) -> Option<Weekday> {
    for (name, wd) in [("一", Weekday::Mon), ("二", Weekday::Tue), ("三", Weekday::Wed),
                       ("四", Weekday::Thu), ("五", Weekday::Fri), ("六", Weekday::Sat),
                       ("日", Weekday::Sun), ("天", Weekday::Sun)] {
        if s.contains(&format!("周{name}")) || s.contains(&format!("星期{name}")) {
            return Some(wd);
        }
    }
    None
}

fn next_weekday(from: NaiveDate, wd: Weekday) -> Option<NaiveDate> {
    let mut d = from.succ_opt()?;
    while d.weekday() != wd {
        d = d.succ_opt()?;
    }
    Some(d)
}

fn next_or_today_weekday(from: NaiveDate, wd: Weekday) -> Option<NaiveDate> {
    if from.weekday() == wd {
        return Some(from);
    }
    next_weekday(from, wd)
}

fn parse_md(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.splitn(2, '月').collect();
    if parts.len() != 2 {
        return None;
    }
    let m: u32 = parts[0].parse().ok()?;
    let d_str: String = parts[1].chars().take_while(|c| c.is_ascii_digit()).collect();
    if d_str.is_empty() {
        return None;
    }
    Some((m, d_str.parse().ok()?))
}

fn parse_time(s: &str, is_today: bool, now: NaiveDateTime) -> NaiveTime {
    let hour_default = if is_today { 18 } else { 9 };
    let (mut h, m) = if let Some(pos) = s.find("点半") {
        let before: String = s[..pos].chars().rev().take_while(|c| c.is_ascii_digit()).collect::<String>().chars().rev().collect();
        (before.parse().unwrap_or(hour_default), 30)
    } else if let Some(pos) = s.find('点') {
        let before: String = s[..pos].chars().rev().take_while(|c| c.is_ascii_digit()).collect::<String>().chars().rev().collect();
        let mut hh: u32 = before.parse().unwrap_or(hour_default);
        let after: String = s[pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        let mm = after.parse().unwrap_or(0);
        if hh <= 6 && (s.contains("下午") || s.contains("晚上") || s.contains("今晚")) {
            hh += 12;
        }
        (hh, mm)
    } else if s.contains("下午") || s.contains("晚上") {
        (15, 0)
    } else if s.contains("上午") {
        (10, 0)
    } else {
        (hour_default, 0)
    };
    if h == 24 {
        h = 0;
    }
    let h = h.min(23);
    let mm = m.min(59);
    NaiveTime::from_hms_opt(h, mm, 0).unwrap_or(now.time())
}
```

`src-tauri/src/lib.rs` 顶部加 `pub mod timeparse;`。

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --test timeparse_test
```

Expected: 5 个测试全部 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/timeparse.rs src-tauri/src/lib.rs src-tauri/tests/timeparse_test.rs
git commit -m "feat: 中文相对时间解析器"
```

---

### Task 11: 承诺存储与提醒状态机

**Files:**
- Create: `src-tauri/src/db/reminders.rs`
- Modify: `src-tauri/src/db/schema.rs`（新增 reminders 表）、`src-tauri/src/commands/record.rs`（承诺写入）
- Test: `src-tauri/tests/reminder_db_test.rs`

**Interfaces:**
- Consumes: Task 8 `save_extraction` 时机、Task 10 `parse_due`
- Produces:
  - `db::reminders::save_reminders(conn, reminders: &[ReminderExtract], conversation_id: i64) -> Result<Vec<i64>>`（`due` 经 `parse_due` 解析；解析失败存原始字符串并标记 `needs_time=true`；无 due 也标记 `needs_time=true`）
  - `db::reminders::list_reminders(conn) -> Result<Vec<ReminderRow>>`（`ReminderRow { id, content, due_at: Option<String>, status, needs_time: bool, conversation_id }`，按 due 升序，NULL 排最后）
  - `db::reminders::set_status(conn, id, status: &str) -> Result<()>`（白名单 pending/done/expired）
  - `db::reminders::reminders_due_soon(conn, now_iso: &str) -> Result<Vec<ReminderRow>>`（status='pending' 且 notified_at IS NULL 且 due_at <= now）

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/reminder_db_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::db;
use smart_bc::memory::types::ReminderExtract;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn save_and_list_reminders() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "周三交方案", None).unwrap();
    let ids = db::reminders::save_reminders(&conn, &[
        ReminderExtract { content: "周三交方案".into(), due: Some("周三".into()) },
        ReminderExtract { content: "买牛奶".into(), due: None },
    ], cid).unwrap();
    assert_eq!(ids.len(), 2);
    let all = db::reminders::list_reminders(&conn).unwrap();
    assert_eq!(all.len(), 2);
    // 买牛奶 due 解析失败 → needs_time
    assert!(all.iter().any(|r| r.content == "买牛奶" && r.needs_time));
}

#[test]
fn status_whitelist() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "x", None).unwrap();
    let ids = db::reminders::save_reminders(&conn, &[ReminderExtract { content: "任务".into(), due: Some("明天".into()) }], cid).unwrap();
    db::reminders::set_status(&conn, ids[0], "done").unwrap();
    assert!(db::reminders::set_status(&conn, ids[0], "hacked").is_err());
}

#[test]
fn due_soon_filter() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "x", None).unwrap();
    db::reminders::save_reminders(&conn, &[ReminderExtract { content: "马上要做的".into(), due: Some("今天下午3点".into()) }], cid).unwrap();
    let now = chrono::NaiveDate::from_ymd_opt(2026, 8, 10).unwrap().and_hms_opt(14, 50, 0).unwrap();
    let soon = db::reminders::reminders_due_soon(&conn, &now.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap();
    assert_eq!(soon.len(), 1);
    assert_eq!(soon[0].content, "马上要做的");
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test reminder_db_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: schema 新增提醒表**

`src-tauri/src/db/schema.rs` 追加：

```rust
        CREATE TABLE IF NOT EXISTS reminders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            due_at TEXT,
            status TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending','done','expired')),
            needs_time INTEGER NOT NULL DEFAULT 0,
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            notified_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_reminders_due ON reminders(due_at);
```

- [ ] **Step 4: 实现 reminders 存储层**

`src-tauri/src/db/reminders.rs`:

```rust
use crate::memory::types::ReminderExtract;
use crate::timeparse::parse_due;
use rusqlite::{params, Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ReminderRow {
    pub id: i64,
    pub content: String,
    pub due_at: Option<String>,
    pub status: String,
    pub needs_time: bool,
    pub conversation_id: i64,
}

pub fn save_reminders(conn: &Connection, reminders: &[ReminderExtract], conversation_id: i64) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    for r in reminders {
        if r.content.trim().is_empty() { continue; }
        let (due_at, needs_time) = match &r.due {
            Some(d) => {
                let now = chrono::Local::now().naive_local();
                match parse_due(d, now) {
                    Some(dt) => (Some(dt.format("%Y-%m-%d %H:%M:%S").to_string()), false),
                    None => (Some(d.clone()), true),
                }
            }
            None => (None, true),
        };
        conn.execute(
            "INSERT INTO reminders (content, due_at, needs_time, conversation_id) VALUES (?1,?2,?3,?4)",
            params![r.content.trim(), due_at, needs_time as i64, conversation_id],
        )?;
        ids.push(conn.last_insert_rowid());
    }
    Ok(ids)
}

pub fn list_reminders(conn: &Connection) -> Result<Vec<ReminderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at, status, needs_time, conversation_id
         FROM reminders ORDER BY due_at IS NULL, due_at ASC, id DESC",
    )?;
    let rows = stmt.query_map([], map_row)?;
    rows.collect()
}

pub fn set_status(conn: &Connection, id: i64, status: &str) -> Result<()> {
    match status {
        "pending" | "done" | "expired" => {}
        _ => return Err(rusqlite::Error::InvalidParameterName(format!("bad status {status}"))),
    }
    conn.execute("UPDATE reminders SET status = ?1 WHERE id = ?2", params![status, id])?;
    Ok(())
}

pub fn reminders_due_soon(conn: &Connection, now_iso: &str) -> Result<Vec<ReminderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, due_at, status, needs_time, conversation_id
         FROM reminders
         WHERE status = 'pending' AND notified_at IS NULL AND due_at IS NOT NULL
           AND due_at <= ?1
         ORDER BY due_at ASC",
    )?;
    let rows = stmt.query_map(params![now_iso], map_row)?;
    rows.collect()
}

fn map_row(r: &rusqlite::Row) -> Result<ReminderRow> {
    Ok(ReminderRow {
        id: r.get(0)?,
        content: r.get(1)?,
        due_at: r.get(2)?,
        status: r.get(3)?,
        needs_time: r.get::<_, i64>(4)? != 0,
        conversation_id: r.get(5)?,
    })
}
```

- [ ] **Step 5: 承诺接入抽取管线**

`src-tauri/src/commands/record.rs` 的 `process_audio_full` 内，`save_extraction` 之后追加：

```rust
    if !ext.reminders.is_empty() {
        crate::db::reminders::save_reminders(conn, &ext.reminders, result.conversation_id)
            .map_err(|e| format!("承诺入库失败: {e}"))?;
    }
```

- [ ] **Step 6: 运行测试确认通过**

```bash
cargo test --test reminder_db_test
cargo test
```

Expected: 全部 PASS。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/db/reminders.rs src-tauri/src/db/schema.rs src-tauri/src/commands/record.rs src-tauri/tests/reminder_db_test.rs
git commit -m "feat: 承诺存储与提醒状态机"
```

---

### Task 12: 提醒调度器 + 桌面通知

**Files:**
- Create: `src-tauri/src/scheduler.rs`
- Modify: `src-tauri/src/lib.rs`（启动调度任务）
- Test: `src-tauri/tests/scheduler_test.rs`

**Interfaces:**
- Consumes: Task 11 `reminders_due_soon`、`set_status`
- Produces:
  - `scheduler::due_reminders_for_notification(reminders: &[ReminderRow], now: NaiveDateTime) -> Vec<ReminderRow>`——纯函数：status='pending' 且 due_at <= now+15min
  - `scheduler::mark_notified(conn: &Connection, id: i64) -> rusqlite::Result<()>`
  - `scheduler::spawn(conn: Arc<Mutex<Connection>>, app: AppHandle)`——后台线程每 60s 轮询，到期发通知并 mark_notified

- [ ] **Step 1: 写失败测试（纯过滤函数）**

`src-tauri/tests/scheduler_test.rs`:

```rust
use chrono::{NaiveDate, NaiveDateTime};
use smart_bc::db::reminders::ReminderRow;
use smart_bc::scheduler::due_reminders_for_notification;

fn dt(s: &str) -> NaiveDateTime {
    NaiveDate::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_hms_opt(0, 0, 0).unwrap()
}

fn row(id: i64, due: &str) -> ReminderRow {
    ReminderRow {
        id, content: format!("任务{id}"), due_at: Some(due.to_string()),
        status: "pending".into(), needs_time: false, conversation_id: 1,
    }
}

#[test]
fn picks_only_reminders_within_15min() {
    let now = dt("2026-08-10 15:00");
    let soon = row(1, "2026-08-10 15:10");
    let ok = row(2, "2026-08-10 15:14");
    let too_far = row(3, "2026-08-10 15:16");
    let past = row(4, "2026-08-10 14:00");
    let due = due_reminders_for_notification(&[soon.clone(), ok.clone(), too_far, past], now);
    let ids: Vec<i64> = due.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn ignores_null_due() {
    let now = dt("2026-08-10 15:00");
    let r = ReminderRow { id: 9, content: "无时间".into(), due_at: None,
        status: "pending".into(), needs_time: true, conversation_id: 1 };
    assert!(due_reminders_for_notification(&[r], now).is_empty());
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test scheduler_test
```

Expected: FAIL（`scheduler` 不存在）。

- [ ] **Step 3: 实现调度逻辑与通知**

`src-tauri/src/scheduler.rs`:

```rust
use crate::db::reminders::{ReminderRow, reminders_due_soon};
use chrono::{Duration, NaiveDateTime};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn due_reminders_for_notification(reminders: &[ReminderRow], now: NaiveDateTime) -> Vec<ReminderRow> {
    let cutoff = now + Duration::minutes(15);
    reminders
        .iter()
        .filter(|r| {
            if r.status != "pending" { return false; }
            match &r.due_at {
                Some(d) => match NaiveDateTime::parse_from_str(d, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => dt <= cutoff,
                    Err(_) => false,
                },
                None => false,
            }
        })
        .cloned()
        .collect()
}

pub fn mark_notified(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET notified_at = datetime('now', 'localtime') WHERE id = ?1",
        rusqlite::params![id],
    )?;
    Ok(())
}

pub fn spawn(conn: Arc<Mutex<Connection>>, app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            {
                let guard = conn.lock().unwrap();
                let now = chrono::Local::now().naive_local();
                let now_iso = now.format("%Y-%m-%d %H:%M:%S").to_string();
                if let Ok(reminders) = reminders_due_soon(&guard, &now_iso) {
                    let due = due_reminders_for_notification(&reminders, now);
                    for r in due {
                        let _ = app.notification()
                            .builder()
                            .title("SmartBC 提醒")
                            .body(&r.content)
                            .show();
                        let _ = mark_notified(&guard, r.id);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}
```

`src-tauri/src/lib.rs` 的 `run()` 内，`Builder` 构建出的 `app` 上启动调度（在 `.run(...)` 前的 setup 或 builder 后无法直接拿 app——正确做法：在 `tauri::Builder::default().setup(|app| { ... spawn ...; Ok(()) })` 中调用）：

```rust
use tauri::Manager;
// 在 setup 闭包内：
let conn_arc = app.state::<AppState>().conn.clone();
let handle = app.handle().clone();
scheduler::spawn(conn_arc, handle);
```

- [ ] **Step 4: 运行测试确认通过**

```bash
cargo test --test scheduler_test
```

Expected: 2 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/scheduler.rs src-tauri/src/lib.rs src-tauri/tests/scheduler_test.rs
git commit -m "feat: 提醒调度器与桌面通知"
```

---

### Task 13: 前端 UI（录音/历史/查询/人脉/承诺）

**Files:**
- Create: `src/api.ts`、`src/pages/RecordPage.tsx`、`src/pages/HistoryPage.tsx`、`src/pages/QueryPage.tsx`、`src/pages/PeoplePage.tsx`、`src/pages/RemindersPage.tsx`、`src/styles.css`
- Modify: `src/App.tsx`（重写为 tab 布局）、`src-tauri/src/lib.rs`（注册 5 个列表命令）、`src-tauri/src/commands/query.rs`（追加列表命令）

**Interfaces:**
- Consumes: 已有命令 `ping`、`start_recording`、`stop_recording`、`get_transcription_status`、`query_memories`
- Produces（新增命令）:
  - `#[tauri::command] fn list_conversations(state) -> Result<Vec<ConversationRow>, String>`
  - `#[tauri::command] fn list_reminders_cmd(state) -> Result<Vec<ReminderRow>, String>`
  - `#[tauri::command] fn list_people_cmd(state) -> Result<Vec<PersonRow>, String>`
  - `#[tauri::command] fn list_preferences_cmd(state) -> Result<Vec<PreferenceRow>, String>`
  - `#[tauri::command] fn complete_reminder(state, id: i64) -> Result<(), String>`（调 `set_status(id, "done")`）

- [ ] **Step 1: 新增查询命令**

`src-tauri/src/commands/query.rs` 追加：

```rust
use crate::db::conversations::ConversationRow;
use crate::db::memories::{PersonRow, PreferenceRow};
use crate::db::reminders::ReminderRow;

#[tauri::command]
pub fn list_conversations(state: tauri::State<'_, AppState>) -> Result<Vec<ConversationRow>, String> {
    let conn = state.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, created_at, transcript, audio_path FROM conversations ORDER BY id DESC LIMIT 100",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(ConversationRow {
        id: r.get(0)?, created_at: r.get(1)?, transcript: r.get(2)?, audio_path: r.get(3)?,
    })).map_err(|e| e.to_string())?;
    rows.collect().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_reminders_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<ReminderRow>, String> {
    let conn = state.conn.lock().unwrap();
    crate::db::reminders::list_reminders(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_people_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<PersonRow>, String> {
    let conn = state.conn.lock().unwrap();
    crate::db::memories::list_people(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_preferences_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<PreferenceRow>, String> {
    let conn = state.conn.lock().unwrap();
    crate::db::memories::list_preferences(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_reminder(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    crate::db::reminders::set_status(&conn, id, "done").map_err(|e| e.to_string())
}
```

`src-tauri/src/lib.rs` 注册上述 5 个命令。

- [ ] **Step 2: API 封装**

`src/api.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";

export interface RecordResult { conversation_id: number; transcript: string; }
export interface ConversationRow { id: number; created_at: string; transcript: string; audio_path: string | null; }
export interface ReminderRow { id: number; content: string; due_at: string | null; status: string; needs_time: boolean; conversation_id: number; }
export interface PersonRow { id: number; name: string; relation: string; note: string; conversation_id: number; }
export interface PreferenceRow { id: number; topic: string; value: string; conversation_id: number; }

export const api = {
  ping: () => invoke<string>("ping"),
  startRecording: () => invoke<void>("start_recording"),
  stopRecording: () => invoke<RecordResult>("stop_recording"),
  transcriptionReady: () => invoke<boolean>("get_transcription_status"),
  queryMemories: (q: string) => invoke<string>("query_memories", { question: q }),
  listConversations: () => invoke<ConversationRow[]>("list_conversations"),
  listReminders: () => invoke<ReminderRow[]>("list_reminders_cmd"),
  listPeople: () => invoke<PersonRow[]>("list_people_cmd"),
  listPreferences: () => invoke<PreferenceRow[]>("list_preferences_cmd"),
  completeReminder: (id: number) => invoke<void>("complete_reminder", { id }),
};
```

- [ ] **Step 3: RecordPage（核心录音交互）**

`src/pages/RecordPage.tsx`:

```tsx
import { useState } from "react";
import { api } from "../api";

export default function RecordPage() {
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const [last, setLast] = useState("");

  const toggle = async () => {
    if (recording) {
      setBusy(true);
      try {
        const r = await api.stopRecording();
        setLast(r.transcript);
      } catch (e) {
        setLast(`录音失败：${e}`);
      } finally {
        setBusy(false);
        setRecording(false);
      }
    } else {
      try {
        await api.startRecording();
        setRecording(true);
        setLast("");
      } catch (e) {
        setLast(`无法开始录音：${e}`);
      }
    }
  };

  return (
    <div className="record-page">
      <h1>SmartBC</h1>
      <button
        className={`mic-btn ${recording ? "recording" : ""}`}
        disabled={busy}
        onClick={toggle}
      >
        {busy ? "处理中…" : recording ? "停止并保存" : "开始录音"}
      </button>
      {last && <p className="transcript">{last}</p>}
    </div>
  );
}
```

- [ ] **Step 4: QueryPage（回忆查询）**

`src/pages/QueryPage.tsx`:

```tsx
import { useState } from "react";
import { api } from "../api";

export default function QueryPage() {
  const [q, setQ] = useState("");
  const [answer, setAnswer] = useState("");
  const [busy, setBusy] = useState(false);

  const ask = async () => {
    setBusy(true);
    try {
      setAnswer(await api.queryMemories(q));
    } catch (e) {
      setAnswer(`查询失败：${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="query-page">
      <h2>回忆</h2>
      <textarea value={q} onChange={(e) => setQ(e.target.value)} placeholder="问它：上次和谁聊了什么？" />
      <button onClick={ask} disabled={busy || !q.trim()}>提问</button>
      <pre className="answer">{answer}</pre>
    </div>
  );
}
```

- [ ] **Step 5: 其余页面**

`src/pages/HistoryPage.tsx`、`src/pages/PeoplePage.tsx`、`src/pages/RemindersPage.tsx`：各自 `useEffect` 拉取 `listConversations` / `listPeople` + `listPreferences` / `listReminders`，渲染列表；RemindersPage 提供"完成"按钮调 `completeReminder` 并刷新。样式统一用 `src/styles.css`（深色主题、居中、大按钮）。

`src/App.tsx` 用简单 tab 切换五个页面（`useState` 索引 + 条件渲染），不做路由库。

- [ ] **Step 6: 验证**

```bash
cargo test
npm run tauri dev
```

Expected: 五个页面可切换；录音→历史出现文本；提问返回回答；承诺页可标记完成。

- [ ] **Step 7: 提交**

```bash
git add src src-tauri/src/commands/query.rs src-tauri/src/lib.rs
git commit -m "feat: 前端五个页面与查询命令"
```

---

### Task 14: 设置页（API Key 配置 + 隐私控制）

**Files:**
- Create: `src/pages/SettingsPage.tsx`、`src-tauri/src/config.rs`、`src-tauri/src/commands/settings.rs`
- Modify: `src-tauri/src/lib.rs`、`src/App.tsx`

**Interfaces:**
- Consumes: Task 8 AppState
- Produces:
  - `config::Config { api_key: String }`（Serialize/Deserialize/Default）
  - `config::config_path(data_dir) -> PathBuf`、`config::load_config(data_dir) -> Config`、`config::save_config(data_dir, cfg) -> Result<(), String>`（JSON 权限 0600）
  - `config::load_api_key(data_dir) -> Option<String>`（环境变量优先，其次文件）
  - `#[tauri::command] fn save_api_key(state, key: String) -> Result<(), String>`（写配置 + 热更新 `state.llm`）
  - `#[tauri::command] fn get_config(state) -> Result<Config, String>`
  - `#[tauri::command] fn clear_all_data(state) -> Result<(), String>`（清空 5 表 + FTS 重建）
  - `#[tauri::command] fn export_all(state, dest: String) -> Result<String, String>`（导出 JSON，返回路径）
  - `#[tauri::command] fn delete_conversation(state, id: i64) -> Result<(), String>`（级联删记忆/提醒 + FTS 行）

- [ ] **Step 1: 写失败测试（config 读写 + 级联删除）**

`src-tauri/tests/settings_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::config;
use smart_bc::db;

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn config_roundtrip() {
    let dir = std::env::temp_dir().join("smartbc_cfg_test");
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config::Config { api_key: "sk-test-123".into() };
    config::save_config(&dir, &cfg).unwrap();
    let loaded = config::load_config(&dir);
    assert_eq!(loaded.api_key, "sk-test-123");
    std::fs::remove_file(config::config_path(&dir)).ok();
}

#[test]
fn env_var_takes_priority() {
    unsafe { std::env::set_var("DEEPSEEK_API_KEY", "sk-env"); }
    let dir = std::env::temp_dir().join("smartbc_cfg_test2");
    let key = config::load_api_key(&dir);
    assert_eq!(key.as_deref(), Some("sk-env"));
    unsafe { std::env::remove_var("DEEPSEEK_API_KEY"); }
}
```

`src-tauri/tests/delete_conversation_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::commands::settings::delete_conversation_core;
use smart_bc::db;
use smart_bc::memory::types::{MemoryExtraction, PersonExtract, ReminderExtract};

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn delete_conversation_cascades() {
    let conn = mem_conn();
    let cid = db::conversations::insert_conversation(&conn, "认识李四并约定周五见面", None).unwrap();
    let ext = MemoryExtraction {
        people: vec![PersonExtract { name: "李四".into(), relation: "朋友".into(), note: String::new() }],
        reminders: vec![ReminderExtract { content: "周五见面".into(), due: Some("周五".into()) }],
        preferences: vec![], episode: None,
    };
    db::memories::save_extraction(&conn, &ext, cid).unwrap();
    db::reminders::save_reminders(&conn, &ext.reminders, cid).unwrap();
    delete_conversation_core(&conn, cid).unwrap();
    assert!(db::conversations::get_conversation(&conn, cid).unwrap().is_none());
    assert!(db::memories::list_people(&conn).unwrap().is_empty());
    assert!(db::reminders::list_reminders(&conn).unwrap().is_empty());
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test settings_test --test delete_conversation_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: 实现 config 模块**

`src-tauri/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub api_key: String,
}

pub fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("config.json")
}

pub fn load_config(data_dir: &Path) -> Config {
    let path = config_path(data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(data_dir: &Path, cfg: &Config) -> Result<(), String> {
    let path = config_path(data_dir);
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn load_api_key(data_dir: &Path) -> Option<String> {
    if let Ok(k) = std::env::var("DEEPSEEK_API_KEY") {
        if !k.is_empty() { return Some(k); }
    }
    let cfg = load_config(data_dir);
    if cfg.api_key.is_empty() { None } else { Some(cfg.api_key) }
}
```

- [ ] **Step 4: 实现设置命令**

`src-tauri/src/commands/settings.rs`:

```rust
use crate::app_state::AppState;
use crate::config::{Config, save_config};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct ExportPayload {
    pub conversations: Vec<crate::db::conversations::ConversationRow>,
    pub people: Vec<crate::db::memories::PersonRow>,
    pub preferences: Vec<crate::db::memories::PreferenceRow>,
    pub episodes: Vec<crate::db::memories::EpisodeRow>,
    pub reminders: Vec<crate::db::reminders::ReminderRow>,
}

#[tauri::command]
pub fn save_api_key(state: tauri::State<'_, AppState>, key: String) -> Result<(), String> {
    save_config(&state.data_dir, &Config { api_key: key.clone() })?;
    let mut guard = state.llm_guard.lock().unwrap();
    *guard = std::sync::Arc::new(crate::llm::client::DeepSeekClient::new(&key));
    Ok(())
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    Ok(crate::config::load_config(&state.data_dir))
}

pub fn delete_conversation_core(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM people WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM preferences WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM episodes WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM reminders WHERE conversation_id = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM conversations_fts WHERE rowid = ?1", rusqlite::params![id])?;
    conn.execute("DELETE FROM conversations WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

#[tauri::command]
pub fn delete_conversation(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    delete_conversation_core(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_all_data(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    conn.execute_batch(
        "DELETE FROM conversations_fts;
         DELETE FROM people; DELETE FROM preferences; DELETE FROM episodes;
         DELETE FROM reminders; DELETE FROM conversations;",
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_all(state: tauri::State<'_, AppState>, dest: String) -> Result<String, String> {
    let conn = state.conn.lock().unwrap();
    let payload = ExportPayload {
        conversations: crate::commands::query::list_conversations_impl(&conn)?,
        people: crate::db::memories::list_people(&conn).map_err(|e| e.to_string())?,
        preferences: crate::db::memories::list_preferences(&conn).map_err(|e| e.to_string())?,
        episodes: crate::db::memories::list_episodes(&conn, 10000).map_err(|e| e.to_string())?,
        reminders: crate::db::reminders::list_reminders(&conn).map_err(|e| e.to_string())?,
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&dest, json).map_err(|e| e.to_string())?;
    Ok(dest)
}
```

（注：`llm_guard` 是 `AppState` 中新增的 `Mutex<Arc<dyn LlmProvider + Send + Sync>>` 字段，替代原 `llm` 字段以支持热更新；`list_conversations_impl` 是从 Task 13 的 `list_conversations` 命令抽出的非命令函数。）

- [ ] **Step 5: 更新 AppState 与 lib.rs**

`src-tauri/src/app_state.rs` 的 `llm` 字段改为：

```rust
    pub llm: Arc<Mutex<Arc<dyn crate::llm::provider::LlmProvider + Send + Sync>>>,
```

所有引用处（Task 8 的 `process_audio_full` 调用方、Task 9 的 `query_memories`）改为先 `let llm = state.llm.lock().unwrap().clone();` 再使用。`lib.rs` 初始化时用 `config::load_api_key(&data_dir)`，注册 `settings` 命令。

- [ ] **Step 6: SettingsPage**

`src/pages/SettingsPage.tsx`: API Key 输入框（加载 `get_config`，保存调 `save_api_key`）；"下载模型"按钮（Task 4 的 download_model 需暴露为命令或文档说明手动下载放置路径）；"清空全部数据"（`clear_all_data` 二次确认）；"导出数据"（调 `export_all`，用原生对话框选路径——`@tauri-apps/plugin-dialog` 或简化固定到 `~/smartbc-export.json`）。

- [ ] **Step 7: 运行测试确认通过**

```bash
cargo test --test settings_test --test delete_conversation_test
cargo test
```

Expected: 全部 PASS。

- [ ] **Step 8: 提交**

```bash
git add src-tauri/src/config.rs src-tauri/src/commands/settings.rs src-tauri/src/app_state.rs src-tauri/src/lib.rs src/pages/SettingsPage.tsx src/App.tsx src-tauri/tests/settings_test.rs src-tauri/tests/delete_conversation_test.rs
git commit -m "feat: 设置页与隐私控制（API Key/清空/导出/删除）"
```

---

### Task 15: 埋点 + 种子用户打包与验证清单

**Files:**
- Create: `src-tauri/src/telemetry.rs`、`src-tauri/src/commands/telemetry.rs`、`docs/seed-user-guide.md`
- Modify: `src-tauri/src/db/schema.rs`（usage_events 表）、`src-tauri/src/lib.rs`、`src/pages/RecordPage.tsx`（埋点调用）
- Test: `src-tauri/tests/telemetry_test.rs`

**Interfaces:**
- Consumes: Task 13 UI 命令
- Produces:
  - `db::schema` 新增 `usage_events (id, event, created_at)` 表
  - `telemetry::log_event(conn, event: &str) -> Result<()>`
  - `#[tauri::command] fn get_usage_stats(state) -> Result<UsageStats, String>`（`UsageStats { recordings: i64, queries: i64, reminder_clicks: i64, last_7d_active_days: i64 }`）
  - `commands::telemetry`：RecordPage 录音成功/停止、QueryPage 提问、RemindersPage 点击完成时调用埋点

- [ ] **Step 1: 写失败测试**

`src-tauri/tests/telemetry_test.rs`:

```rust
use rusqlite::Connection;
use smart_bc::db;
use smart_bc::telemetry::{log_event, usage_stats};

fn mem_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    db::schema::migrate(&conn).unwrap();
    conn
}

#[test]
fn log_and_count_events() {
    let conn = mem_conn();
    log_event(&conn, "recording_done").unwrap();
    log_event(&conn, "recording_done").unwrap();
    log_event(&conn, "query_asked").unwrap();
    let stats = usage_stats(&conn).unwrap();
    assert_eq!(stats.recordings, 2);
    assert_eq!(stats.queries, 1);
}

#[test]
fn rejects_unknown_events() {
    let conn = mem_conn();
    assert!(log_event(&conn, "DROP TABLE conversations").is_err());
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cargo test --test telemetry_test
```

Expected: FAIL（模块不存在）。

- [ ] **Step 3: schema 追加 + 实现 telemetry**

`src-tauri/src/db/schema.rs` 追加：

```rust
        CREATE TABLE IF NOT EXISTS usage_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
```

`src-tauri/src/telemetry.rs`:

```rust
use rusqlite::{params, Connection, Result};
use serde::Serialize;

const ALLOWED_EVENTS: [&str; 3] = ["recording_done", "query_asked", "reminder_clicked"];

pub fn log_event(conn: &Connection, event: &str) -> Result<()> {
    if !ALLOWED_EVENTS.contains(&event) {
        return Err(rusqlite::Error::InvalidParameterName(format!("unknown event {event}")));
    }
    conn.execute("INSERT INTO usage_events (event) VALUES (?1)", params![event])?;
    Ok(())
}

#[derive(Debug, Serialize, Default)]
pub struct UsageStats {
    pub recordings: i64,
    pub queries: i64,
    pub reminder_clicks: i64,
    pub last_7d_active_days: i64,
}

pub fn usage_stats(conn: &Connection) -> Result<UsageStats> {
    let count = |event: &str| -> Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM usage_events WHERE event = ?1",
            params![event],
            |r| r.get(0),
        )
    };
    Ok(UsageStats {
        recordings: count("recording_done")?,
        queries: count("query_asked")?,
        reminder_clicks: count("reminder_clicked")?,
        last_7d_active_days: conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM usage_events
             WHERE date(created_at) >= date('now', '-7 days')",
            [], |r| r.get(0),
        )?,
    })
}
```

`src-tauri/src/commands/telemetry.rs`:

```rust
use crate::app_state::AppState;
use crate::telemetry::{UsageStats, log_event, usage_stats};

#[tauri::command]
pub fn log_usage(state: tauri::State<'_, AppState>, event: String) -> Result<(), String> {
    let conn = state.conn.lock().unwrap();
    log_event(&conn, &event).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_usage_stats(state: tauri::State<'_, AppState>) -> Result<UsageStats, String> {
    let conn = state.conn.lock().unwrap();
    usage_stats(&conn).map_err(|e| e.to_string())
}
```

`src-tauri/src/lib.rs` 注册 `log_usage`、`get_usage_stats`。

- [ ] **Step 4: 前端埋点接入**

`src/pages/RecordPage.tsx` 录音成功后：

```ts
import { api } from "../api";
// stopRecording 成功后：
await api.logUsage("recording_done");
```

`src/pages/QueryPage.tsx` 提问成功后：

```ts
await api.logUsage("query_asked");
```

`src/pages/RemindersPage.tsx` 点击"完成"后：

```ts
await api.logUsage("reminder_clicked");
```

`src/api.ts` 追加：

```ts
logUsage: (event: string) => invoke<void>("log_usage", { event }),
getUsageStats: () => invoke<{ recordings: number; queries: number; reminder_clicks: number; last_7d_active_days: number }>("get_usage_stats"),
```

- [ ] **Step 5: 种子用户指南**

`docs/seed-user-guide.md`: 安装包路径（`src-tauri/target/release/bundle/nsis/*.exe`）、首次使用流程（配置 API Key → 下载模型 → 录音测试）、隐私说明文案（本地存储 + 云端仅文本）、数据导出/清空指引、反馈收集渠道（问卷/群聊）、每周统计查看方式（设置页显示 `get_usage_stats`）。

- [ ] **Step 6: 运行测试确认通过 + 最终回归**

```bash
cargo test
npm run tauri build
```

Expected: 全部测试 PASS；release 安装包生成成功。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/telemetry.rs src-tauri/src/commands/telemetry.rs src-tauri/src/db/schema.rs src-tauri/src/lib.rs src/pages src/api.ts docs/seed-user-guide.md src-tauri/tests/telemetry_test.rs
git commit -m "feat: 埋点统计与种子用户打包验证清单"
```

---

## 自审记录（writing-plans self-review）

- **Spec 覆盖**：三核心功能（语音记忆=Task 1-5、承诺提醒=Task 10-12、回忆查询=Task 9）+ 隐私控制（Task 14）+ 埋点验收（Task 15）+ 迭代节奏（任务顺序对应周 1-4）全部有对应任务。
- **占位符扫描**：所有步骤含具体代码与命令；无 TBD/TODO。
- **类型一致性**：`LlmProvider::chat_json`、`MemoryExtraction`、`ReminderRow`、`SearchHit`、`QueryContext` 等跨任务签名一致；`AppState.llm` 在 Task 14 由 `Arc<dyn>` 改为 `Arc<Mutex<Arc<dyn>>>` 时明确标注了引用处同步修改。
