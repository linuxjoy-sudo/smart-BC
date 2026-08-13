# SmartBC：唤醒词拼音匹配 — 设计文档

- 日期：2026-08-14
- 状态：草稿（待用户审阅）
- 定位：解决 whisper-small 对中文唤醒词"小贝小贝"的同音字识别不稳定（转写成"小杯小杯"、"小辈小辈"、"小备小备"等导致匹配失败）

## 1. 背景与问题

实测日志显示（00:46 版本）：
- 唤醒词"小贝小贝"被 whisper 转写成多种同音/近音形式："小备小备"、"小费﹑小费"、"小杯小杯"、"小贝"（截断）
- 现有 `contains_wake_word` 做**字符级连续子串匹配**（已过滤标点/空白），对同音字混淆无能为力——转写文本本身就不是"小贝小贝"

根因：whisper-small 对"贝"(bèi) 的同音/近音字（杯 bēi、备 bèi、辈 bèi、费 fèi）选择不稳定；"小贝小贝"无语义上下文，模型倾向输出常见字。

## 2. 方案选型

| 方案 | 结论 |
|---|---|
| A. 同音字族映射表（归一化） | 备选：需手写同音字表，覆盖不全 |
| **B. 拼音匹配**（选定） | 纯 Rust `pinyin` crate，转无声调拼音后包含匹配，对同音字完全鲁棒 |
| C. 换 medium 模型 | 转写 4.5s→15s，唤醒检测不可接受 |
| D. 换唤醒词 | 改变品牌词，不理想 |

## 3. 设计

### 3.1 依赖
`pinyin = "0.11.0"`（纯 Rust、无 C 依赖、Windows 可编译；默认 `compat` feature 含 `plain`）

### 3.2 wake.rs 改造

```rust
use pinyin::{Pinyin, ToPinyin};

fn pinyin_key(text: &str) -> String {
    text.to_pinyin().flatten().map(Pinyin::plain).collect()
}

pub fn contains_wake_word(text: &str, wake_word: &str) -> bool {
    if text.is_empty() || wake_word.is_empty() { return false; }
    let text_key = pinyin_key(text);
    let wake_key = pinyin_key(wake_word);
    if wake_key.is_empty() { return false; }
    text_key.contains(&wake_key)
}
```

- `to_pinyin()`：中文逐字转拼音（Option），非中文（标点/空白/字母）返回 None 被 `flatten` 跳过
- `Pinyin::plain()`：无声调拼音（bei、bei、bei 同音字归并）
- 匹配：拼音串包含关系——"小杯小杯"(xiaobeixiaobei) 包含 "小贝小贝"(xiaobeixiaobei) ✓

### 3.3 已知边界
- "小费小费"(xiaofeixiaofei) 不匹配（b/f 声母混淆，whisper 罕见误识别，接受）
- 多音字：唤醒词"小贝"无多音，通用文本多音字取拼音库第一读音，不影响唤醒匹配
- 误唤醒：任何 bei 音 4 字组合会命中，但作为自然语言罕见，风险低

## 4. 测试计划

wake_test.rs 更新：
- 同音变体命中：小杯小杯 / 小辈小辈 / 小备小备 / 小北小北 / 小贝﹑小贝（标点）
- 无关词拒绝：明天几点开会
- 空/纯空白唤醒词拒绝
- 小费小费不匹配（b/f 边界，明确预期）

## 5. 明确假设

- 拼音匹配替换字符匹配（不做双轨）
- `pinyin` crate 默认读音（非多音字全枚举）
- 接受"小费"等 b/f 混淆的罕见失败
