# SmartBC 需求状态追踪

> 活文档：每次功能开发/验证后更新。状态：`完成` / `进行中` / `待批准` / `候选`。
> 详细功能与用法见 [README](../../README.md)。

## 一、代码功能总结

**架构**：Tauri 2 + Rust（后端）+ React/TS（前端）+ SQLite，Windows 桌面应用，核心交互为语音。

### 后端（Rust，约 1600 行核心逻辑）

| 模块 | 功能 |
|---|---|
| `voice/dialog.rs` | 语音状态机核心：Idle→唤醒→Active 聆听→断句→转写→记录/问答/补时间/提醒响应 |
| `voice/listener.rs` | cpal 音频采集（含立体声去交错，修复 Realtek 多声道音频错乱） |
| `voice/vad.rs` | 能量 VAD（10ms 帧、起音 3 帧去抖/结束 10 帧迟滞） |
| `voice/wake.rs` | 唤醒词匹配：拼音 + 送气声母容错 + 模糊匹配（容错同音/插入字/声母混淆） |
| `voice/tts.rs` | Windows WinRT TTS 播报（含回声抑制标志，播报期间暂停麦克风处理） |
| `voice/reply.rs` | 播报分发（notification/voice/both） |
| `asr/whisper.rs` | whisper 转写：beam search(5)、音量归一化、no_speech 拒识、48k→16k 重采样 |
| `memory/extract.rs` | LLM 抽取（人脉/偏好/事件/提醒）+ 提醒内容清洗（去"提醒我/叫我/帮我"前缀） |
| `timeparse.rs` | 中文自然语言时间解析（"3分钟后"/"明天9点"/"周五晚上"，中文数字） |
| `scheduler.rs` | 提醒调度：30s 循环扫描 + tokio 精确定时器、去重、过期流转、语音响应共享状态 |
| `db/` | SQLite（conversations/people/preferences/episodes/reminders，FTS5 检索，schema 迁移） |

### 前端（6 页面）
录音、问答、历史（显示 LLM 语义摘要）、人脉、承诺、设置。

## 二、需求状态列表

| # | 需求 | 状态 | 提交 | 备注 |
|---|---|---|---|---|
| 1 | 录音转写（whisper、设备选择、音量归一化、简繁转换） | ✅ 完成 | 早期 + `2ed1a4d` | |
| 2 | 语音助手唤醒链路（信号链修复：VAD 去抖/拒识/裁剪/节流） | ✅ 完成 | `82067ed` `f53ee9b` | |
| 3 | 立体声输入去交错（Realtek 多声道音频错乱） | ✅ 完成 | `44f2baf` | |
| 4 | 唤醒词鲁棒性（送气声母容错"小沛"、"小贝的小贝"模糊匹配） | ✅ 完成 | `554b1f5` `a1c44e8` | |
| 5 | 第2次唤醒提速（Active 用 base 快速检测唤醒词） | ✅ 完成 | `4576e94` | |
| 6 | 回声抑制（TTS 播报期间不误识别） | ✅ 完成 | `7d946fb` | |
| 7 | 提醒不触发修复（extract prompt 引导相对时间 + ISO 兜底 + 调度循环化） | ✅ 完成 | `18476b9` `9dcf68e` | 17:01 准时触发验证 |
| 8 | 转写质量提升（greedy→beam search） | ✅ 完成 | `c8e4572` | 小贝小贝.m4a 精准识别 |
| 9 | P1 无时间提醒补时间 → 语音交互追问（"什么时候提醒你？"） | ✅ 完成 | `7ed37d2` | 已按用户要求改为语音为主 |
| 10 | P2 已通知超 1 天提醒自动流转 expired | ✅ 完成 | `1c4c994` | |
| 11 | P3 通知可交互 → 语音交互（提醒后说"完成"/"延后5分钟"） | ✅ 完成 | `6fec99b` | 已按用户要求改为语音为主 |
| 12 | 播报助理口吻 + 内容清洗（"到时间了，该做早餐了"） | ✅ 完成 | `6fec99b` `5519c87` | |
| 13 | 历史页显示 LLM 语义摘要（summary 列，与通知一致） | ✅ 完成 | `9dcf68e` | |
| 14 | 麦克风未拾音误报修复（阈值 0.005→0.002） | ✅ 完成 | `18476b9` | |
| 15 | README 功能/用法文档 | ✅ 完成 | `854028a` | |
| 16 | GitHub 私有仓库推送 | ✅ 完成 | — | https://github.com/linuxjoy-sudo/smart-BC（后改为公开） |
| 17 | CI 流水线（GitHub Actions：快速回归 voice_chain + 全量 + 构建） | ✅ 完成 | `a7214ef` `29d1add` `91b7815` `ff98a0d` | 3 Job 全绿验证（快速回归/全量+clippy/Windows 构建）；设计见 [ci-pipeline-design.md](ci-pipeline-design.md) |
| 18 | 方案 B：离线录音→ASR→全链路测试（voice_chain_test） | ✅ 完成 | `3a5344c` | base 模型 + TTS fixtures（wake.wav/reminder.wav）真实 ASR 转写→唤醒断言→process_transcript；CI Job1 自动运行验证 |
| 19 | 方案 D1：AudioFeed/DialogSink 抽象 + 进程内状态机测试（dialog_loop_test） | ✅ 完成 | `6bb1fbf` | run_loop 可注入 WavFeed/MockSink；2 项测试真实 ASR（唤醒触发/非唤醒保持 Idle）；CI Job1 启用 |
| 20 | 方案 C：VB-CABLE 开发机 E2E 冒烟脚本 | ⬜ 待实施 | — | CI Phase 3，不进 CI |
| 21 | 去 GUI 化（方案 A+C）：主窗口默认隐藏 + 系统托盘 + 首次引导 + 语音系统指令 | ✅ 完成 | `e4023c5` | 用户确认完成 |
| 22 | 提醒补发依次播报（TTS 串行队列 + 带原定时间文案） | ✅ 完成 | `bfde979` | 重启后多条过期提醒逐个播报不叠加 |
| 23 | 转写质量提升：重采样升级（rubato sinc）+ ASR 模型可切换（small/medium） | ✅ 完成 | `bfde979` | medium 评估：真实录音从乱码→清晰语义；config.asr_model 切换（默认 small 保实时） |
| 24 | 语音调用系统应用（B 级：启动应用/URL/搜索 + 音量/媒体键） | ✅ 完成 | `7e8d6b2` | 白名单映射 + config app_commands 可覆盖；音量 Core Audio/媒体键 SendInput；12 项解析测试；设计见 specs/2026-08-16-launch-apps-design.md |

### 已实测验证
- ✅ 唤醒功能正常（多次实测）
- ✅ 提醒准时触发（17:01:05 验证）＋ 语音完成/延后
- ✅ 转写准确率提升（beam）

## 三、候选 / 未批准（后续可评估）

| 需求 | 状态 | 说明 |
|---|---|---|
| 提醒提前通知语义确认 | ⬜ 候选 | 保留 |
| ~~界面语音优先精简~~ | ❌ 废弃 | 被 #21 去 GUI 化吸收 |
| ~~转写质量进一步提升~~ | ✅ 转正 | #23 已完成（medium 可切换 + sinc 重采样） |
## 四、更新指引

每次功能迭代后更新本文件：
1. 新功能 → 在"需求状态列表"追加一行（状态/提交/备注）
2. 候选项被批准 → 移到"需求状态列表"标记"进行中"，完成后改"完成"
3. 已批准未实施的需求 → 标记"进行中"
