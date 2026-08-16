# 语音调用系统应用（B 级）设计

> 日期：2026-08-16
> 需求：#24（进行中）
> 范围：启动应用（L1）+ 带参数/搜索（L2）+ 系统控制：音量/媒体键（L4）

## 目标

语音助手可通过自然语言启动系统内应用、打开网页/搜索、调节音量、控制媒体播放。交互全部语音（延续去 GUI 化方向）。

## 语音指令设计

### 新增 SystemCommand 变体（voice/commands.rs）

| 语音 | 指令 | 示例播报 |
|---|---|---|
| "打开计算器" / "启动记事本" | `LaunchApp(String)` | "好的，已打开计算器" |
| "用记事本打开 D:\笔记.txt" | `LaunchWith(app, path)` | "好的，用记事本打开" |
| "搜索今天天气" / "查一下天气" | `Search(String)` | "好的，已搜索今天天气" |
| "打开 bing.com" | `OpenUrl(String)` | "好的，已打开网页" |
| "调高音量" / "音量调到50" | `Volume(f32)`（增量/绝对值） | "音量已调到 50" |
| "静音" / "取消静音" | `Mute(bool)` | "已静音" |
| "播放" / "暂停" / "下一首" / "上一首" | `MediaPlayPause/MediaNext/MediaPrev` | "已播放" |

### 解析规则（parse_system_command 扩展）

优先级：现有指令（查看历史/承诺/人脉/状态/设备）→ 新指令。

- `打开X` / `启动X` / `开启X` → LaunchApp(X)，X 从应用映射表解析
- `用X打开Y` → LaunchWith(X, Y)
- `搜索Z` / `查一下Z` / `帮我搜Z` → Search(Z)
- `打开网址` + URL → OpenUrl（校验 http/https）
- `调高/调低音量`、`音量调到N`、`大点声/小点声` → Volume
- `静音`/`取消静音`/`恢复声音` → Mute
- `播放`/`暂停`/`下一首`/`上一首`/`继续播放` → Media 系列

## 应用映射

### 内置常用表（voice/apps.rs）

| 名称 | 命令/路径 |
|---|---|
| 计算器 | calc.exe |
| 记事本 | notepad.exe |
| 画图 | mspaint.exe |
| 控制面板 | control |
| 我的电脑 | explorer |
| 命令提示符 | cmd |
| 回收站 | explorer shell:RecycleBinFolder |
| 音乐 / 媒体播放器 | wmplayer.exe（Windows 媒体播放器） |
| 浏览器 | 默认浏览器（opener） |

### config.json 扩展（用户自定义）

```json
"app_commands": {
  "网易云音乐": "D:\\Software\\Netease\\cloudmusic.exe",
  "QQ音乐": "C:\\Program Files\\Tencent\\QQMusic\\QQMusic.exe"
}
```

启动时合并：内置表 + config 覆盖/新增。

### 配置能力（更新音乐播放器等）

用户可通过编辑 config.json 的 `app_commands` 自定义/覆盖应用映射：

| 场景 | 行为 |
|---|---|
| config 用**同名 key**（如"音乐"） | **覆盖**内置条目（如 wmplayer.exe → 网易云音乐） |
| config 用**新名称**（如"网易云音乐"） | 内置表 + 新增，两者都可用 |
| 不配置 | 用内置默认 |

- **路径写法**：双反斜杠 `D:\\Software\\cloudmusic.exe` 或正斜杠 `D:/Software/cloudmusic.exe` 均可；含空格路径直接执行
- **生效方式**：修改后**重启应用**（启动时加载合并映射表）
- 播报提示：应用未注册时提示"可在设置里配置"（引导用户加 app_commands）

### 搜索

默认浏览器打开 `https://www.bing.com/search?q=<urlencoded>`（系统默认搜索引擎不可编程获取，用 Bing）。

## 执行引擎（新模块 voice/launch.rs）

| 操作 | 实现 |
|---|---|
| 启动应用 | `std::process::Command::new(path).spawn()`（白名单路径） |
| 打开 URL | `opener` crate（默认浏览器） |
| 音量调节 | `windows` crate Core Audio：`IAudioEndpointVolume` 的 `SetMasterVolumeLevelScalar` / `SetMute` |
| 媒体键 | `SendInput` 模拟 `VK_MEDIA_PLAY_PAUSE(0xB3)` / `NEXT_TRACK(0xB0)` / `PREV_TRACK(0xB1)` |

音量/媒体为 `#[cfg(windows)]`（Linux 编译走空实现或返回错误）。

## 安全边界（策略 A：允许列表）

- **应用白名单**：仅内置表 + config `app_commands`；语音无法启动未注册程序
- **URL 协议校验**：仅 `http://` / `https://`（拒绝本地命令/文件协议）
- **文件路径校验**：`LaunchWith` 的目标文件必须存在（不存在 → 播报失败）

## 错误处理与播报

| 场景 | 播报 |
|---|---|
| 应用未注册 | "没有找到应用「X」，可在设置里配置" |
| 启动失败 | "打开失败：原因" |
| 文件不存在 | "找不到文件：路径" |
| URL 非法 | "不支持打开这个地址" |
| 音量/媒体成功 | "音量已调到 50" / "已静音" / "已播放" |

播报走现有 `DialogSink`（MockSink 可测）。

## 依赖

- `windows`（features: Core-Audio, Win32_UI_Input_KeyboardAndMouse, Win32_System_Com）
- `opener`

## 测试

- `voice_commands_test` 扩展：新指令解析（打开/搜索/音量/静音/媒体）
- 应用映射查询（纯函数）：内置 + config 合并
- URL 协议校验（纯函数）
- 音量/媒体键：Windows 特有，`#[cfg(windows)]` + 手动验证

## 非目标

- 应用内部自动化（UIAutomation）——L3，后续候选
- 亮度调节、应用级音量——高复杂度，不做
- 任意命令执行——安全策略 A 禁止
