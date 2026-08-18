# 语音全双工交互改造设计

> 日期：2026-08-17
> 需求：#25（进行中）
> 状态：已确认方案（流式 ASR sherpa-onnx + WebRTC AEC + 打断 + 连续对话）

## 目标

将当前半双工语音交互（唤醒→聆听→断句→转写→TTS→等回应）改造为**全双工**：边说边识别、播报可打断、对话中免唤醒持续聆听，接近真人对讲体验。

## 当前架构（半双工）与限制

```
用户说话 → VAD聆听 → 1.5s静音断句 → whisper转写(2-5s) → LLM处理 → TTS播报
                                              ↑                            ↓
                                     （播报期间麦克风抑制防回声）      用户听完再说
```

限制：不能打断播报、需重复唤醒、转写延迟 2-5s。

## 目标架构（全双工）

```
┌────────────────────────────────────────────────────────────┐
│                      持续音频链路                            │
│  麦克风(cpal) ──┐                                            │
│                 ├─→ [WebRTC AEC] ─→ [VAD] ─→ [sherpa-onnx]  │
│  TTS 参考信号 ──┘           │           │       流式识别      │
│  (自渲染 PCM)                │           │       partial/final│
│                             │           └─→ 对话引擎         │
│  [打断检测]：AEC后语音 → stop_tts ──→ 聆听用户新指令          │
│                                                             │
│  TTS(WinRT→流) → PCM → cpal播放 ──→ AEC参考 + 可中断          │
└────────────────────────────────────────────────────────────┘
```

## 组件改造

### 1. 流式 ASR（sherpa-onnx）

- 选型：sherpa-onnx（paraformer 中文流式模型，~300MB，实时词条延迟 200-500ms）
- 替代：现有 whisper 整段转写（保留用于唤醒检测？或统一 sherpa）
- 输出：partial（实时词条）+ final（完整句）
- 模型获取：hf-mirror 下载 paraformer 模型

### 2. AEC 回声消除（webrtc-audio-processing）

- 输入：麦克风信号 + TTS 播放参考信号（PCM）
- 输出：消除回声后的纯人声 → VAD/打断检测
- 前提：TTS 必须自渲染 PCM（见下）

### 3. TTS 改造（Qwen3-TTS，首选）

- **Qwen3-TTS（Rust crate，candle 推理）**：
  - 流式合成（`synthesize_streaming` chunk 输出）——边说边播，与流式 ASR 对称
  - 完全可控（播放循环可中断、自渲染 PCM 可直接送 AEC 参考）
  - 中文支持、9 预设声音、声音克隆（Base 模型）
  - 模型：0.6B（CPU 可跑）/ 1.7B（高质量）；`qwen3-tts = { version = "0.1" }`
- **WinRT TTS 保留为 fallback**（模型未下载/低资源模式）
- **可中断**：播放循环检查打断标志 → 停止播放

### 4. 对话状态机

```
[Idle] --唤醒词"小贝小贝"--> [对话中]
  ▲                            │
  │                   流式识别持续聆听（免唤醒）
  │                   打断：播报中说话 → 停TTS → 听新指令
  │                   退出：静默10s 或 "先这样/再见"
  └────────────────────────────┘
```

- 唤醒进入：保留唤醒词（当前 wake.rs 匹配逻辑）
- 对话中：免唤醒持续聆听（VAD + 流式识别）
- 打断：播报中 AEC 后检测到语音 → 停止 TTS → 进入聆听
- 退出：静默 10s 或语音指令（"先这样"/"再见"）

### 5. 处理引擎

- final 完整句 → 现有 process_transcript 链路（LLM 抽取/问答/提醒）
- partial 词条 → 可选实时反馈（如"正在听..."）

## 技术依赖

| 依赖 | 用途 |
|---|---|
| `sherpa-onnx` | 流式 ASR（paraformer 模型） |
| `qwen3-tts`（candle） | 流式 TTS（首选，可中断+送 AEC 参考） |
| WinRT tts crate | TTS fallback（模型未下载/低资源） |
| `webrtc-audio-processing` | AEC 回声消除 |
| cpal 输出 | TTS 自播放 |

## 风险与非目标

- 延迟目标：词条 200-500ms，完整句 1-2s（优于 whisper 2-5s）
- 非目标：多说话人分离、背景噪声抑制（AEC 含基本 NS）
- 模型下载 ~300MB（hf-mirror 可拉）
- Qwen3-TTS 0.6B CPU 推理延迟需评估（首 chunk 响应时间）
- Qwen3-TTS 模型下载 ~1-2GB（hf-mirror）
- WinRT fallback 保证模型缺失时播报可用

## 测试策略

- 流式识别：合成音频流式喂入 → 断言 partial/final 时序
- AEC：合成"播报回声+人声"混合 → 断言人声保留
- 打断：模拟播报中语音 → 断言 TTS 停止
- 全链路：WavFeed 流式回放 → 状态机断言（扩展 dialog_loop_test）

## 实施阶段（建议）

1. **Phase 1**：sherpa-onnx 接入（流式识别替代断句转写）+ 模型下载评估
2. **Phase 2**：Qwen3-TTS 接入（流式合成 + cpal 播放 + 可中断；WinRT 保留 fallback）
3. **Phase 3**：WebRTC AEC 接入（打断检测）
4. **Phase 4**：对话状态机（唤醒进入/免唤醒/静默退出）+ 全链路测试

## 非目标（后续候选）

- Qwen3-TTS 声音克隆调优（Base 模型，后续）
- 多设备/远场唤醒
