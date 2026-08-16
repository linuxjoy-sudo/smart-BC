# CI 流水线设计

> 活文档：GitHub Actions 流水线设计。状态：`进行中`（Phase 1 实施中）。
> 仓库：https://github.com/linuxjoy-sudo/smart-BC（公开）

## 设计目标

- **每次变更自动回归**（替代"部署前手动全量"）
- **分层反馈**：秒级快速层 → 分钟级全量层，失败快反馈
- **覆盖真实声学层**（方案 B：voice_chain_test）+ **状态机时序层**（方案 D1：dialog_loop_test）
- 平台：GitHub Actions

## 流水线总览

```
触发: push(master) / pull_request
┌─────────────────────────────────────────────────────┐
│  Job 1: 快速回归 (ubuntu-latest, ~1min)             │
│    voice_chain_test  (B: 真实音频→whisper→DB)        │
│    dialog_loop_test  (D1: 进程内状态机时序)          │
│    wake/vad/timeparse 针对性                        │
├─────────────────────────────────────────────────────┤
│  Job 2: 全量测试 (ubuntu-latest, 2-5min)            │
│    cargo test (22文件 110+ 测试)                     │
│    cargo clippy --all-targets (零警告)               │
│    cargo check                                       │
├─────────────────────────────────────────────────────┤
│  Job 3: 构建 (windows-latest, ~10min)               │
│    npm run tauri build -- --no-bundle                │
│    (发布产物上传 artifact)                           │
├─────────────────────────────────────────────────────┤
│  Job 4: 冒烟 (开发机手动触发, 不进CI)               │
│    C: VB-CABLE + PowerShell 真 E2E (场景1-5)        │
│    场景6 回声抑制 (reply_mode=voice, 仅开发机)       │
└─────────────────────────────────────────────────────┘
```

## Job 详解

### Job 1：快速回归（ubuntu-latest）

```yaml
steps:
  - checkout + rust-toolchain
  - apt install cmake                    # whisper-rs 依赖
  - 下载模型 → actions/cache 缓存         # base + small
  - cargo cache (~/.cargo + target)
  - env: SMARTBC_MODEL_DIR=models
    run: cargo test --test voice_chain_test --test dialog_loop_test
  - run: cargo test --test wake_test --test vad_test --test timeparse_test
```

- **voice_chain_test**（方案 B）：测试音频 wav → whisper 转写 → 断言唤醒命中/内容 → MockProvider 全链路 → DB
- **dialog_loop_test**（方案 D1）：`AudioFeed::WavFeed` 实时喂入 → 完整状态机（唤醒→断句→补时间→完成/延后 + TTS 暂停期）
- **门控**：缺模型自动 skip；模型命中缓存秒级加载

### Job 2：全量测试（ubuntu-latest）

```yaml
run: cargo test            # 22 文件 110+ 测试（纯逻辑层）
run: cargo clippy --all-targets -- -D warnings
```

- 覆盖现有全部模块（唤醒/VAD/时间/调度/DB/抽取/问答/配置...）
- **零警告硬门禁**

### Job 3：构建（windows-latest）

```yaml
run: npm run tauri build -- --no-bundle
- uses: actions/upload-artifact  # smartbc.exe 供下载
```

- 目标平台 Windows，产物直接可取
- 构建期间 Job 1/2 已并行跑完

### Job 4：开发机冒烟（手动，不在 CI）

- `scripts/e2e-windows.ps1`：VB-CABLE 虚拟麦克风 + 真实播放 + 日志/DB 断言
- 场景 6（回声抑制）需真实 TTS 播放，仅开发机
- 作用：真人发声 → 提醒到点 → 语音响应的**最终 E2E**，替代高频人工实测

## 缓存策略

| 缓存 | Key | 命中效果 |
|---|---|---|
| whisper 模型 | `whisper-models-v1` | 465MB 下载→秒级 |
| cargo/target | `cargo-${{ hashFiles('Cargo.lock') }}` | 首次编译 5-10min→30s |
| 构建产物 | upload-artifact | 直接下载部署 |

## 门控与失败语义

| 层级 | 失败处理 |
|---|---|
| Job 1 | PR 标记失败（快反馈，~1min） |
| Job 2 | 全量回归拦截（不合并） |
| Job 3 | 构建产物不可用（不发布） |
| Job 4 | 手动触发，不影响 CI 绿 |

## 模型与测试资产

- **whisper 模型**（ggml-base / ggml-small）：上传至 GitHub Releases 资产，workflow 下载 + 缓存
  - 模型为开源许可（whisper.cpp），公开仓库可分发
- **测试音频**（fixtures）：
  - ⚠️ 项目已**公开**——**真实用户语音录音不得入库/上传公开 Releases**（隐私）
  - 仓库内测试音频仅允许：TTS 合成音频（如用 Windows TTS 合成"小贝小贝"）或非敏感合成音
  - 真实录音（recordings/、小贝小贝.m4a）仅存本地/私有存储，本地开发跑完整 voice_chain

## 与本地流程的分工

```
本地（WSL）      针对性测试 → clippy → 构建 → 部署 → 用户实测
CI               Job1 快速回归 → Job2 全量 → Job3 构建
开发机（可选）    Job4 真 E2E 冒烟（替代高频人工）
```

- **CI 兜底回归**（每次变更必跑全量），本地保持快速迭代
- **人工实测收窄**到最终验收抽查 + Job4 冒烟

## 演进路径

1. **Phase 1**（当前）：Job 1 的 voice_chain_test（B，合成音频）+ Job 2/3 上线
2. **Phase 2**：D1 重构（AudioFeed trait）→ dialog_loop_test 入 Job 1
3. **Phase 3**：Job 4 脚本（C），开发机按需

## 相关需求

- 需求状态见 [requirements-status.md](./requirements-status.md)（#17 CI 流水线）
