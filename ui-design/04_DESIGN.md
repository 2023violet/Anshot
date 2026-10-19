# 04 DESIGN.md — ShareReady「暗夜工作台」

版本 v0.1 草稿　日期 2026-10-19　状态：**待 Gate 04 冻结**（依赖 Gate 03 框架 spike）
来源：[03_VISUAL_DIRECTIONS.md](03_VISUAL_DIRECTIONS.md) —— Raycast 骨架 + Linear 强调色规则。样稿：[03_VISUAL_SAMPLE.html](03_VISUAL_SAMPLE.html)

## 1. 设计原则

1. **截图是主角**：界面是深色中性框架，不和截图内容抢颜色。
2. **一种强调色**：lavender-blue 只表示“你正在操作的东西”（当前工具、选中、焦点）。
3. **饱和色只属于内容**：标注颜色只出现在画布上的标注和色板里，不出现在界面框架上。
4. **靠亮度分层，不用阴影**：深度来自表面色阶和 1px 发丝线。
5. **键盘可见**：每个工具都显示 keycap 快捷键；焦点永远可见。
6. **紧凑但不拥挤**：桌面工具密度，13px 正文，32px 控件。
7. **遮挡有安全身份**：遮挡工具使用盾牌图标和独立的 secure 色，与标注区分。

可访问性目标：正文对比度 ≥ 4.5:1，图标与焦点环 ≥ 3:1；颜色不是唯一的状态表达。

## 2. Token

### 2.1 颜色 — 界面框架

| Token | 值 | 用途 | 来源 |
|---|---|---|---|
| `bg.workspace` | `#07080a` | 画布后的工作区底色 | Raycast canvas |
| `bg.chrome` | `#0d0d0d` | 工具栏、状态栏 | Raycast surface |
| `bg.elevated` | `#101111` | 属性条、输入框、弹出菜单 | Raycast surface-elevated |
| `bg.card` | `#121212` | keycap、提示、对话框 | Raycast surface-card |
| `bg.hover` | `#18191a` | 按钮悬停 | Raycast button-fg |
| `bg.pressed` | `#1f2022` | 按钮按下 | 推导 |
| `line.hairline` | `#242728` | 所有 1px 分隔与边框 | Raycast hairline |
| `line.strong` | `rgba(255,255,255,0.16)` | 图片外框、强分隔 | Raycast hairline-strong |
| `text.ink` | `#f4f4f6` | 标题、选中文字 | Raycast ink |
| `text.body` | `#cdcdcd` | 默认文字、图标 | Raycast body |
| `text.mute` | `#9c9c9d` | 次要信息、状态栏 | Raycast mute |
| `text.disabled` | `#6a6b6c` | 禁用 | Raycast ash |
| `accent` | `#5e6ad2` | 当前工具底色、选中框、焦点环、手柄描边 | Linear primary |
| `accent.hover` | `#828fff` | 强调色悬停 | Linear primary-hover |
| `accent.soft` | `rgba(94,106,210,0.18)` | 选中行、多选区域填充 | 推导 |
| `secure` | `#7a7fad` | 遮挡工具选中底色、遮挡对象边框 | Linear brand-secure |
| `primary.bg` | `#ffffff` | 唯一主按钮“复制” | Raycast primary |
| `primary.pressed` | `#e8e8e8` | 主按钮按下 | Raycast primary-pressed |
| `primary.fg` | `#000000` | 主按钮文字 | Raycast on-primary |
| `ok` | `#59d499` | 成功提示图标 | Raycast accent-green |
| `error` | `#ff6161` | 错误提示、危险操作文字 | Raycast accent-red |
| `warn` | `#ffc533` | 警告 | Raycast accent-yellow |
| `scrim` | `rgba(0,0,0,0.55)` | 对话框遮罩、截图覆盖层暗区 | 推导 |

### 2.2 颜色 — 内容（标注与遮挡）

| Token | 值 | 键 |
|---|---|---|
| `ink.red` | `#f2463c` | 1（默认） |
| `ink.yellow` | `#ffd60a` | 2 |
| `ink.green` | `#30c45a` | 3 |
| `ink.blue` | `#2f7cf6` | 4 |
| `ink.white` | `#ffffff` | 5 |
| `ink.black` | `#111111` | 6 |
| `redact.solid` | `#111111`（可选 `#ffffff`） | 实心遮挡默认色 |
| `redact.mosaic` | `#3a3b3e` `#4a4b4f` `#5b5c61` `#6c6d72` | 安全马赛克生成调色板（与原图无关） |

文字标注的“背景底色”选项使用当前标注色 + 白/黑自动对比文字。

### 2.3 字体

- 拉丁：**Inter**（OFL，打包）；中文：**Noto Sans CJK SC**（OFL，仓库已有 `poc/native-egui/assets/fonts/`）。不依赖系统字体。
- `ss03` 等 OpenType 特性：可选，取决于框架能力（HYPOTHESIS）。
- 数字（尺寸、坐标、缩放）使用 tabular 数字（若框架支持），否则使用 Inter 默认。

| Token | 字号 / 字重 / 行高 | 用途 |
|---|---|---|
| `type.title` | 14 / 600 / 20 | 对话框标题、空状态标题 |
| `type.body` | 13 / 400 / 20 | 默认文字、按钮标签、菜单 |
| `type.body.strong` | 13 / 500 / 20 | 当前工具标签、提示主句 |
| `type.caption` | 12 / 400 / 16 | 状态栏、属性条标签、提示说明 |
| `type.key` | 11 / 500 / 14 | keycap |
| `type.hero` | 20 / 600 / 28 | 仅空状态主标题 |
| 标注文字 | 默认 18，可选 14 / 18 / 24 / 32 / 48，字重 600 | 画布内容 |

字距：正文 0，keycap +0.2px。不使用负字距。

### 2.4 间距、尺寸、圆角

- 间距：`2 4 8 12 16 24 32`（base 4）。
- 控件高：图标按钮 **32×32**；文字按钮 32 高、左右 12 内距；输入框 28 高；keycap 18 高。
- 结构高：工具栏 **48**；属性条 **40**；状态栏 **28**；标题栏使用系统标题栏。
- 圆角：`r.xs 4`（keycap、徽标）、`r.sm 6`（图标按钮、色块）、`r.md 8`（文字按钮、输入、菜单）、`r.lg 10`（对话框、提示、浮层）。不使用胶囊按钮。
- 线宽：界面 1px；选中框 1px；手柄描边 1.5px；焦点环 2px（外扩 2px）。
- 最小命中区：32×32；画布手柄可视 8×8、命中区 16×16。

### 2.5 层级

| 层 | 表现 | 用途 |
|---|---|---|
| 0 | `bg.workspace` | 画布背景 |
| 1 | `bg.chrome` + 底部或顶部 1px `line.hairline` | 工具栏、状态栏 |
| 2 | `bg.elevated` + 1px `line.hairline` | 属性条、菜单 |
| 3 | `bg.card` + 1px `line.strong` | 对话框、提示（Toast） |

不使用投影。唯一例外：截图覆盖层的选区外使用 `scrim` 暗化。

### 2.6 动效

- 悬停 / 按下：背景色过渡 **80ms** ease-out。
- 属性条出现 / 切换：高度与透明度 **140ms** ease-out。
- 提示进出：上移 8px + 淡入 **160ms**；3 秒后淡出。
- 画布缩放：以指针为中心，**120ms** 插值；连续滚轮时不插值。
- 无弹跳、无循环动画。系统关闭动画时全部改为即时。

## 3. 布局规则（对应 02 的 S0–S5）

- **工具栏（48）**：左侧 工具组（标注 6 个 ｜ 遮挡 2 个 ｜ 裁剪 1 个），组间 1px 竖线 + 8px 间距；右侧 撤销/重做、保存（次要文字按钮）、复制（主按钮）。按钮之间 4px。
- **属性条（40）**：`bg.elevated`，左对齐，内容按 02 §3 随上下文变化；无内容时收起，画布上移。
- **画布**：`bg.workspace` 上居中放图，图片外 1px `line.strong`；不加棋盘格（截图没有透明）；PNG 含透明时才显示 8px 深色棋盘格 `#121212/#18191a`。
- **状态栏（28）**：`type.caption` `text.mute`；左：尺寸 · 已遮挡 N 处（图标盾牌 `secure`）；右：缩放控件。
- **窗口宽 < 900**：工具按钮隐藏文字只留图标 + 悬停提示；**宽 < 720**：保存按钮收为图标。最小 640×420。
- **截图覆盖层**：选区外 `scrim`；选区 1px `#ffffff` 边 + 8 个手柄；尺寸标签 `bg.card` + `type.caption`；放大镜 120×120，`r.lg`，1px `line.strong`，中心十字 `accent`。

## 4. 组件规格

### 4.1 按钮
| 组件 | 默认 | 悬停 | 按下 | 选中 | 禁用 |
|---|---|---|---|---|---|
| 工具按钮（图标 32） | 透明，图标 `text.body` | `bg.hover` | `bg.pressed` | `accent` 底，图标 `#fff` | 图标 `text.disabled` |
| 遮挡工具按钮 | 同上 | 同上 | 同上 | **`secure` 底**，图标 `#fff` | 同上 |
| 主按钮“复制” | `primary.bg`，`primary.fg`，`r.md`，`type.body.strong`，带 `Ctrl C` keycap（深色） | 不变 | `primary.pressed` | — | `bg.elevated` + `text.disabled` |
| 次要按钮 | 透明，1px `line.strong`，`text.ink` | `bg.hover` | `bg.pressed` | — | `text.disabled`，线 `line.hairline` |
| 危险文字按钮（删除、放弃编辑） | 透明，`error` 文字 | `bg.hover` | `bg.pressed` | — | — |

焦点：所有可聚焦组件 2px `accent` 外环，间隔 2px，圆角随组件 +2。

### 4.2 keycap
`bg.card` → `#0d0d0d` 竖向渐变（若框架不支持渐变则纯 `bg.card`），1px `line.hairline`，`r.xs`，`type.key` `text.mute`，高 18，左右 5 内距。出现在：工具悬停提示、主按钮、空状态、菜单右侧。

### 4.3 色板（属性条）
色块 20×20，`r.sm`；选中：外圈 2px `text.ink` + 间隔 2px；白色色块加 1px `line.strong`。右侧 `type.caption` 显示数字键。

### 4.4 分段控件（粗细、遮挡样式）
容器 `bg.chrome` + 1px `line.hairline`，`r.md`，高 28；项透明 `text.body`；选中项 `bg.card` + `text.ink`（Raycast pill-tab-active 的方角版本）。

### 4.5 输入框（尺寸、字号）
`bg.chrome`，1px `line.hairline`，`r.md`，高 28，`type.body`；聚焦边框 `accent`；错误边框 `error` + 下方 `type.caption` 说明。

### 4.6 画布对象
- 选中框：1px `accent`；手柄 8×8 `#ffffff` 填充 + 1.5px `accent` 描边，`r` 2。
- 悬停可选对象：1px `accent.hover` 50% 透明虚线。
- 遮挡对象（未选中）：编辑时在边缘画 1px `secure` 虚线，便于识别；**导出时不包含**这条线。
- 多选框：`accent.soft` 填充 + 1px `accent`。
- 裁剪：外部 `scrim`；裁剪框 1px `#fff` + 三分线 `rgba(255,255,255,0.3)`。

### 4.7 提示（Toast）
`bg.card`，1px `line.strong`，`r.lg`，内距 10/14，最小宽 240；左侧 16px 状态图标（`ok`/`error`/`warn`），文字 `type.body.strong` `text.ink`，附加说明 `type.caption` `text.mute`；操作按钮为次要按钮小号（高 26）。位置：主显示器右下，距边 16。

### 4.8 对话框
`bg.card`，1px `line.strong`，`r.lg`，宽 400，内距 20；标题 `type.title`；正文 `type.body` `text.body`；按钮右对齐，间距 8：取消（次要）、放弃（危险文字）、主操作（主按钮）。遮罩 `scrim`。Esc = 取消，Enter = 主操作。

### 4.9 菜单（托盘、右键）
`bg.elevated`，1px `line.hairline`，`r.md`，内距 4；行高 30，`r.sm`，悬停 `bg.hover`；右侧 keycap；分隔 1px `line.hairline`。

### 4.10 悬停提示（Tooltip）
延迟 500ms；`bg.card`，1px `line.hairline`，`r.sm`，内距 4/8；“工具名 + keycap”。

### 4.11 空状态（S3）
三个入口卡片 160×112，`bg.chrome`，1px `line.hairline`，`r.lg`；图标 24 `text.body`；标题 `type.body.strong`；keycap 居中。下方拖放区 1px 虚线 `line.strong`，`r.lg`，拖入时边框 `accent` + `accent.soft` 填充。

## 5. 交互状态总表

| 状态 | 规则 |
|---|---|
| Hover | 背景提升一级（透明 → `bg.hover`）；不变色、不放大 |
| Focus | 2px `accent` 外环（键盘聚焦时显示；鼠标点击不显示） |
| Pressed | `bg.pressed` 或 `primary.pressed` |
| Selected | 工具：`accent` 底（遮挡工具 `secure` 底）；对象：选中框 + 手柄 |
| Disabled | 文字/图标 `text.disabled`；不响应悬停 |
| Loading | 画布中央 20px 环形进度（`text.mute`），旧内容保持显示；按钮内进度时标签不变宽 |
| Empty | S3 规格 |
| Error | Toast `error` + 可重试；输入框 `error` 边框 |
| Success | Toast `ok`；复制成功后窗口关闭（02 U07） |

## 6. 图标

- 线性图标，24 网格、1.5px 线宽，圆角端点；在 32 按钮中绘制 18px。
- 来源候选：Lucide（ISC 许可）导出为 SVG/字形打包。许可需在 06 前复核（NOT_VERIFIED）。
- 工具图标：选择 `mouse-pointer-2`、箭头 `move-up-right`、框 `square`、文字 `type`、序号 `circle-1` 自绘、高亮 `highlighter`、遮挡 `shield`、马赛克 `grid-3x3`+盾牌角标、裁剪 `crop`。

## 7. Rust 主题落地（草案）

```rust
// apps/desktop/src/ui/theme.rs（06 阶段创建；框架无关的 token 常量）
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

pub mod color {
    use super::Rgba;
    pub const BG_WORKSPACE: Rgba = Rgba(0x07, 0x08, 0x0a, 0xff);
    pub const BG_CHROME:    Rgba = Rgba(0x0d, 0x0d, 0x0d, 0xff);
    pub const BG_ELEVATED:  Rgba = Rgba(0x10, 0x11, 0x11, 0xff);
    pub const BG_CARD:      Rgba = Rgba(0x12, 0x12, 0x12, 0xff);
    pub const LINE_HAIRLINE: Rgba = Rgba(0x24, 0x27, 0x28, 0xff);
    pub const ACCENT:       Rgba = Rgba(0x5e, 0x6a, 0xd2, 0xff);
    pub const SECURE:       Rgba = Rgba(0x7a, 0x7f, 0xad, 0xff);
    // …其余 token 与 §2 表一一对应
}
pub mod size { pub const TOOLBAR_H: f32 = 48.0; pub const CONTROL: f32 = 32.0; /* … */ }
```

- 内容色（§2.2）属于产品语义，放在 `shareready-editor`（标注颜色枚举），因为它们会进入导出像素；界面 token 只在 GUI crate。
- 框架适配层把这些常量映射为 egui `Visuals` / iced `Theme` / Slint 全局属性，具体在 Gate 03 选定后写。

## 8. 验收方法

- 每个组件在真实应用中截图，与 [03_VISUAL_SAMPLE.html](03_VISUAL_SAMPLE.html) 并排对照（100% 与 150% DPI）。
- 对比度用取色值计算（§1 目标）。
- 变更记录：本文件末尾“修订记录”，版本号递增。

## 修订记录
- v0.1（2026-10-19）初稿：Raycast + Linear 融合。

---

## Gate 04 检查
- [x] Token、排版、密度、组件、交互状态、适配规则已定义
- [ ] Gate 03 框架 spike 后确认 token 可在原生框架实现
- [ ] **用户确认冻结基线**：＿＿＿＿
