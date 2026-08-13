# SmartBC：唤醒延迟优化 — 设计文档

- 日期：2026-08-14
- 状态：草稿（待用户审阅）
- 定位：唤醒词响应延迟 4-5.5s（偶发 14-15s）优化至 1.5-2.5s，不损失唤醒精度

## 1. 背景与目标

实测唤醒转写耗时多数 4.0-5.5s（输入音频 1.3-3.3s），偶发 14-15s。Oracle 咨询结论：
- 延迟主因：small 模型前向推理（85-92%），非状态分配/重采样
- 尖峰根因：CPU 被抢占或热降频（固定 ≤5s 输入慢 3x，只能是推理本身被拖慢）

目标：唤醒响应 4.5s → 1.5-2.5s（~2 倍速），问答路径（small）精度零影响。

## 2. 方案（Oracle 推荐 S1+S2+S3+S4，本期实施 S1+S2+S3，S4 待 benchmark）

| 方案 | 内容 | 预期提速 | 精度风险 |
|---|---|---|---|
| **S1 分级模型** | 唤醒路径用 base-q5（142MB），问答保留 small | 唤醒推理 ~2x | 低（base + 拼音兜底）；需夹具验证 |
| **S2 State 复用** | listener 持一个 WhisperState 复用（whisper-rs 0.16 owned 类型，可复用） | 50-200ms/次 + 减尖峰 | 无 |
| **S3 静音裁剪** | transcribe 入口 RMS 窗口裁掉前导静音 | 15-25% | 无（只裁静音） |
| S4 线程 benchmark | 4→物理核数，实测决定 | 1.2-1.5x | 无（待用户实测后定） |

## 3. 组件改动

### 3.1 S2 State 复用（whisper.rs）
```rust
pub fn new_state(&self) -> Result<WhisperState, String> {
    self.ctx.create_state().map_err(|e| e.to_string())
}
pub fn transcribe_with_state(&self, state: &mut WhisperState, rate: u32, samples: &[f32]) -> Result<String, String>
// transcribe_samples 保留（内部 new_state + 转调），供 record 路径
```
dialog.rs：`let mut wake_state = transcriber.new_state()?;` 一次，循环里复用。

### 3.2 S3 静音裁剪（whisper.rs transcribe_with_state 入口）
对 `mono` 做 10ms 帧 RMS 扫描，裁掉首段连续低于 0.01 的帧。

### 3.3 S1 分级模型
- `asr/model.rs`：`WAKE_MODEL_FILENAME = "ggml-base.bin"` + hf-mirror URL
- `AppState`：加 `wake_transcriber: Arc<Mutex<Option<Transcriber>>>`
- `lib.rs`：读 `cfg.wake_model`（已有字段），"base" 时加载 base；空/缺失回退主 transcriber（向后兼容）
- `dialog.rs`：Idle 唤醒路径用 wake_transcriber（base）；Active 问答路径仍用主 transcriber（small）
- 设置页：复用 wake_model 字段 UI（可后续加）

## 4. 精度夹具（前置硬约束）

录 15-20 条真实"小贝小贝" + 10 条否定句 → `tests/fixtures/` → 测试跑 `Transcriber + contains_wake_word` 断言命中/未命中。用户提供录音后补齐。

## 5. 风险与回退

- base 首次下载 142MB：启动时不存在优雅回退 small
- `wake_model` 死字段接线时 fallback 主 transcriber，避免旧配置空串导致唤醒无模型
- WhisperState 复用不可跨并发（full 取 &mut self）：record 路径继续走 transcribe_samples 自建 state
- base 精度不足 → 回退 small 或调整唤醒词

## 6. 验证

- cargo test 全量（覆盖新增代码、行覆盖 ≥80%）
- 实测唤醒延迟分布（中位 + P95）对比前后
