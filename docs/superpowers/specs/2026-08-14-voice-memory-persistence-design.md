# SmartBC：语音助手记忆沉淀（方案 A）— 设计文档

- 日期：2026-08-14
- 状态：草稿（待用户审阅）
- 定位：修复语音助手"只能查不能记"的缺口——语音对话转写后入库 + 抽取记忆 + 识别记录/提问意图

## 1. 背景与实证

用户实测（13:20 日志）：
```
你说："明天中午12点提醒我去吃饭。"
回答："我还没有这方面的记忆。你希望我记住这个提醒吗？..."
```
语音助手路径（dialog.rs）转写后**仅 RAG 问答**，不入库、不抽取记忆——"提醒我""记一下"类陈述被当问句回答，LLM 主动建议设置提醒但系统不创建。与手动录音（完整沉淀）不一致。

## 2. 方案 A：双路径合一

语音助手断句转写后，复用手动录音的记忆沉淀链路：

```
whisper 转写文本
  → store_transcript            入库 conversations（audio_path=None）
  → extract_from_transcript     LLM 抽取（people/reminders/preferences/episode）
  → save_extraction/save_reminders  记忆/提醒入库
  → 判断意图：
      ├─ 抽取到实质内容（reminders/people/preferences/episode 任一非空）
      │    → 记录确认通知（"已设置提醒：..."），不 RAG 回答
      └─ 全部为空 → 视为提问 → answer_question_core（RAG 回答）
```

### 2.1 意图判断依据
LLM 抽取器已能识别"提醒/记录"语义（extract_from_transcript）：
- 抽到 reminders → "提醒我..."类指令 → 创建提醒 + 确认
- 抽到 people/preferences/episode → "张伟是供应商"类陈述 → 保存 + 确认
- 全部为空 → 纯提问（"明天几点开会"）→ RAG 回答
- 抽取失败 → 回退 RAG 回答（不阻断）

## 3. 组件改动

### 3.1 新增可测试纯函数（voice/dialog.rs 或新模块）
```rust
pub enum TranscriptOutcome {
    Recorded(String),   // 记录确认文本
    Answered(String),   // RAG 回答文本
}

pub fn process_transcript(
    conn: &rusqlite::Connection,
    llm: &dyn LlmProvider,
    text: &str,
) -> Result<TranscriptOutcome, String>
```
- 复用：`store_transcript`、`extract_from_transcript`、`save_extraction`、`save_reminders`
- 返回 Recorded(确认文本) 或 Answered(回答文本)

### 3.2 dialog.rs 断句分支接入
转写 Ok → 调 `process_transcript` → 通知 Recorded/Answered 文本 → 状态转换

### 3.3 测试
- 纯函数单测（mock LLM + 内存 DB）：
  - "明天12点提醒我去吃饭" → Recorded（reminders 非空）
  - "张伟是供应商" → Recorded（people 非空）
  - "明天几点开会" → Answered（全部为空）
  - 抽取失败 → Answered 回退

## 4. 风险与边界

- 入库空文本：store_transcript 拒绝空文本（已处理）
- 语音助手对话与手动录音重复入库？——各自独立 conversation，正常
- 提醒去重：同一句话多次说会建多个提醒（与手动录音一致，暂不去重）
- 通知文本：Recorded 时显示确认（"已设置提醒：..."），不再返回"我还没有这方面的记忆"

## 5. 验证

- cargo test 全量（新增纯函数测试）
- 实测：说"提醒我明天12点吃饭" → 收到"已设置提醒"通知 + 提醒入库（到点通知）
