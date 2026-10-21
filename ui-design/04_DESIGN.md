# 04 DESIGN.md — ShareReady「奶油编辑台」

> 命名说明：本文写于临时代号 ShareReady 时期，文中旧名与旧路径（如 `crates/shareready-*`）保留原样；2026-10-21 起产品名为 **安截 Anshot**，代码在 `crates/anshot-*`，仓库为 <https://github.com/2023violet/Anshot>。

版本 **v1.0（冻结基线）**　日期 2026-10-19　状态：**Gate 04 已冻结（用户，2026-10-19）**；GUI 框架 Slint 1.18.1。变更需递增版本并记录。
来源：[03_VISUAL_DIRECTIONS.md](03_VISUAL_DIRECTIONS.md) v0.2 —— Cursor 暖奶油编辑风为主，Raycast keycap。样稿：[03_VISUAL_SAMPLE.html](03_VISUAL_SAMPLE.html)

## 1. 设计原则

1. **温度即个性**：暖奶油底、暖黑文字，拒绝纯白纯黑；辨识度来自色温与排版，不靠装饰。
2. **一种品牌电压**：Cursor 橙只给“复制”、焦点环和字标；其余“当前/选中”用暖黑反色。
3. **截图是主角**：界面安静，画布上的截图与标注色最鲜明。
4. **粉彩只表达状态**：薄荷、蜜桃、金、粉蓝胶囊只出现在状态栏与提示，不做按钮或装饰。
5. **安全有颜色**：遮挡相关一律用深绿 + 盾牌，与标注明确区分。
6. **杂志语气**：标题字重 400、轻微负字距；数值用等宽字体，显得精确。
7. **发丝线分层，无投影**。

可访问性目标：正文 ≥ 4.5:1；图标、边框、焦点环 ≥ 3:1；颜色不是唯一的状态表达。§2 中的对比度为实测计算值（MEASURED，WCAG 公式）。

## 2. Token

### 2.1 颜色 — 界面

| Token | 值 | 用途 | 来源 / 对比度 |
|---|---|---|---|
| `bg.canvas` | `#f7f7f4` | 工具栏、状态栏、窗口底 | Cursor canvas |
| `bg.soft` | `#fafaf7` | 属性条 | Cursor canvas-soft |
| `bg.card` | `#ffffff` | 卡片、对话框、提示、菜单、输入框 | Cursor surface-card |
| `bg.workspace` | `#ecebe5` | 画布后工作区（比 canvas 深一级，让白色截图浮起） | 推导 |
| `bg.hover` | `#efeee8` | 悬停 | Cursor hairline-soft |
| `bg.pressed` | `#e6e5e0` | 按下、分段控件选中项 | Cursor surface-strong |
| `line.hairline` | `#e6e5e0` | 1px 分隔 | Cursor hairline |
| `line.strong` | `#cfcdc4` | 卡片外框、截图外框、次要按钮边框 | Cursor hairline-strong |
| `text.ink` | `#26251e` | 标题、按钮文字、选中 | Cursor ink · 14.3:1 |
| `text.body` | `#5a5852` | 正文、图标 | Cursor body · 6.6:1 |
| `text.mute` | `#6b6960` | 次要信息、状态栏文字 | 推导（Cursor muted 仅 3.84:1，不达标）· 5.1:1 |
| `text.decor` | `#807d72` | 装饰性图标、分隔符号（非文字） | Cursor muted · 3.8:1 |
| `text.disabled` | `#a09c92` | 禁用 | Cursor muted-soft |
| `select.bg` | `#26251e` | 当前工具底、选中反色 | Cursor featured tier 反色 |
| `select.fg` | `#f7f7f4` | 反色上的图标/文字 | 14.3:1 |
| `brand` | `#f54e00` | 字标、键盘焦点环 | Cursor primary · 3.3:1（非文字） |
| `primary.bg` | `#d04200` | 主按钮“复制”底色 | Cursor primary-active；白字 4.7:1 |
| `primary.pressed` | `#b33900` | 主按钮按下 | 推导 |
| `primary.fg` | `#ffffff` | 主按钮文字 | — |
| `secure` | `#1f8a65` | 遮挡工具角标、遮挡对象虚线、盾牌图标 | Cursor success · 4.0:1（非文字） |
| `error` | `#cf2d56` | 错误图标与文字、危险操作 | Cursor error |
| `scrim` | `rgba(38,37,30,0.55)` | 对话框遮罩、截图覆盖层暗区 | 暖黑推导 |

### 2.2 颜色 — 状态胶囊（只用于状态栏、提示）

| Token | 底色 | 文字 | 含义 | 来源 |
|---|---|---|---|---|
| `pill.secure` | `#9fc9a2` | `text.ink`（8.3:1） | 已遮挡 N 处 | Cursor timeline-grep（薄荷） |
| `pill.busy` | `#dfa88f` | `text.ink` | 生成中 / 处理中 | Cursor timeline-thinking（蜜桃） |
| `pill.done` | `#c08532` | `text.ink`（4.9:1；Cursor 原用白字仅 3.2:1） | 已复制 / 已保存 | Cursor timeline-done（金） |
| `pill.info` | `#9fbbe0` | `text.ink` | 提示 / 只读 | Cursor timeline-read（粉蓝） |

### 2.3 颜色 — 内容（标注与遮挡）

| Token | 值 | 键 |
|---|---|---|
| `ink.red` | `#e5342b` | 1（默认；与品牌橙拉开色相） |
| `ink.yellow` | `#ffc60a` | 2 |
| `ink.green` | `#22a652` | 3 |
| `ink.blue` | `#2f6ff0` | 4 |
| `ink.white` | `#ffffff` | 5 |
| `ink.black` | `#1a1915` | 6 |
| `redact.solid` | `#1a1915`（可选 `#ffffff`） | 实心遮挡默认色 |
| `redact.mosaic` | `#3a3934` `#4b4a44` `#5d5b54` `#6f6d65` | 安全马赛克生成调色板（暖灰，与原图无关） |

### 2.4 字体

- 拉丁：**Inter**；中文：**Noto Sans CJK SC**；数值：**JetBrains Mono**。均 OFL，随应用打包，不依赖系统字体。（Cursor 原字体 CursorGothic 为商业授权，不使用。）
- 标题负字距按 Cursor 的“Inter 替代 −1.5%”规则；框架不支持字距时退为 0（HYPOTHESIS，spike 验证）。

| Token | 字号 / 字重 / 行高 / 字距 | 用途 |
|---|---|---|
| `type.display` | 26 / 400 / 32 / −0.4px | 空状态主标题 |
| `type.title` | 15 / 600 / 22 / 0 | 对话框标题 |
| `type.body` | 13 / 400 / 20 / 0 | 默认文字、菜单 |
| `type.button` | 13 / 500 / 16 / 0 | 按钮标签 |
| `type.caption` | 12 / 400 / 16 / 0 | 属性条标签、提示说明 |
| `type.pill` | 11 / 600 / 14 / +0.6px | 状态胶囊（中文不加字距） |
| `type.mono` | 12 / 400 / 16 / 0（JetBrains Mono） | 尺寸、坐标、缩放、keycap |
| 标注文字 | 默认 18，可选 14 / 18 / 24 / 32 / 48；字重 600 | 画布内容 |

### 2.5 间距、尺寸、圆角

- 间距：`4 8 12 16 20 24 32`（base 4，Cursor 刻度）。
- 控件：图标按钮 **32×32**；文字按钮高 32、左右 14；输入框高 28；keycap 高 18；胶囊高 20。
- 结构：工具栏 **52**；属性条 **40**；状态栏 **30**；系统标题栏。
- 圆角：`r.xs 4`（keycap）、`r.sm 6`（图标按钮、色块）、`r.md 8`（文字按钮、输入、菜单行）、`r.lg 12`（卡片、对话框、提示、菜单）、`r.pill`（仅状态胶囊）。
- 线宽：界面 1px；画布选中框 1px 暖黑 + 1px 白外线；手柄描边 1.5px；焦点环 2px `brand`，外扩 2px。
- 命中区：控件 ≥ 32×32；画布手柄可视 8×8、命中 16×16。

### 2.6 层级

| 层 | 表现 | 用途 |
|---|---|---|
| 0 | `bg.workspace` | 画布背景 |
| 1 | `bg.canvas` + 1px `line.hairline` | 工具栏、状态栏 |
| 2 | `bg.soft` + 1px `line.hairline` | 属性条 |
| 3 | `bg.card` + 1px `line.strong` | 截图、卡片、对话框、提示、菜单 |

不使用投影。唯一例外：截图覆盖层选区外的 `scrim`。

### 2.7 动效

- 悬停 / 按下背景过渡 **80ms** ease-out。
- 属性条出现 / 切换 **140ms**（高度 + 透明度）。
- 提示：上移 8px + 淡入 **160ms**，3 秒后淡出；状态胶囊颜色切换 **120ms**。
- 画布缩放以指针为中心 **120ms** 插值；连续滚轮不插值。
- 无弹跳、无循环动画；系统关闭动画时即时切换。

## 3. 布局规则（对应 02 的 S0–S5）

- **工具栏（52）**：左侧品牌字标（橙色小方块 + “ShareReady”，`type.button`）→ 工具组（标注 6 ｜ 遮挡 2 ｜ 裁剪 1），组间 1px 竖线；右侧 撤销 / 重做、保存（次要）、复制（主）。按钮间距 4。
- **属性条（40）**：`bg.soft`，随上下文显示（02 §3）；无内容时收起。
- **画布**：`bg.workspace` 上居中放图；图片外 1px `line.strong`；透明 PNG 才显示 8px 浅棋盘格 `#ffffff/#efeee8`。
- **状态栏（30）**：左：尺寸（`type.mono`）· `pill.secure`「已遮挡 N 处」；中：交付状态胶囊（生成中 / 已复制）；右：缩放（`type.mono`）。
- **窗口宽 < 900**：工具只留图标；**< 720**：字标只留橙色方块，保存收为图标。最小 640×420。
- **截图覆盖层**：选区外 `scrim`；选区 1px 白 + 1px 暖黑双线 + 8 手柄；尺寸标签 `bg.card` + `type.mono`；放大镜 120×120 `r.lg`，中心十字 `brand`。

## 4. 组件规格

### 4.1 按钮
| 组件 | 默认 | 悬停 | 按下 | 选中 | 禁用 |
|---|---|---|---|---|---|
| 工具按钮（图标 32） | 透明，图标 `text.body` | `bg.hover` | `bg.pressed` | `select.bg` 底，图标 `select.fg` | `text.disabled` |
| 遮挡工具按钮 | 同上；右下 6px `secure` 盾牌角标 | 同上 | 同上 | `select.bg` 底 + 角标保持 `secure`（亮版 `#5fd3a6`） | 同上 |
| 主按钮“复制” | `primary.bg`，`primary.fg`，`r.md`，`type.button`，内嵌浅色 keycap | `#bf3d00` | `primary.pressed` | — | `bg.pressed` + `text.disabled` |
| 次要按钮 | `bg.card`，1px `line.strong`，`text.ink` | `bg.hover` | `bg.pressed` | — | `text.disabled` |
| 危险文字按钮 | 透明，`error` | `bg.hover` | `bg.pressed` | — | — |

焦点：所有可聚焦组件 2px `brand` 外环，间隔 2px（仅键盘聚焦时显示）。

### 4.2 keycap
`bg.card`，1px `line.strong`，底边 2px `line.strong`（实体键感），`r.xs`，`type.mono` 11px `text.mute`，高 18，左右 5 内距。主按钮内版本：`rgba(255,255,255,0.18)` 底、白字、无边框。

### 4.3 色板
色块 20×20，`r.sm`；白色块 1px `line.strong`；选中：外圈 2px `text.ink`，间隔 2px；右侧 `type.mono` 数字键。

### 4.4 分段控件（粗细、遮挡样式）
容器 `bg.pressed`，`r.md`，高 28，内距 2；项透明 `text.body`；选中项 `bg.card` + 1px `line.strong` + `text.ink`。

### 4.5 输入框
`bg.card`，1px `line.strong`，`r.md`，高 28，`type.mono`；聚焦边框 `brand`；错误边框 `error` + 下方 `type.caption` `error`。

### 4.6 状态胶囊
`r.pill`，高 20，左右 8，`type.pill`；可带 12px 图标（盾牌、勾、环形进度）。只在状态栏和提示中使用。

### 4.7 画布对象
- 选中框：1px `select.bg` + 外侧 1px `#ffffff`；手柄 8×8 白填充 + 1.5px `select.bg` 描边，圆角 2。
- 悬停可选对象：1px 暖黑 40% 虚线。
- 遮挡对象（编辑态）：边缘 1px `secure` 虚线 + 左上 12px 盾牌标记；**导出时不包含**。
- 多选框：`rgba(38,37,30,0.08)` 填充 + 1px `select.bg`。
- 裁剪：外部 `scrim`；裁剪框 1px 白 + 三分线 `rgba(255,255,255,0.5)`。

### 4.8 提示（Toast）
`bg.card`，1px `line.strong`，`r.lg`，内距 10/14，最小宽 260；左侧状态胶囊（`pill.done`「已复制」等）或错误图标；主句 `type.button` `text.ink`，说明 `type.caption` `text.mute`（尺寸用 `type.mono`）；操作为次要按钮小号（高 26）。主显示器右下距边 16。

### 4.9 对话框
`bg.card`，1px `line.strong`，`r.lg`，宽 400，内距 24；标题 `type.title`；正文 `type.body`；按钮右对齐间距 8：取消（次要）、放弃（危险）、主操作（主按钮）。遮罩 `scrim`。Esc 取消，Enter 主操作。

### 4.10 菜单（托盘、右键）
`bg.card`，1px `line.strong`，`r.lg`，内距 4；行高 30、`r.md`，悬停 `bg.hover`；右侧 keycap；分隔 1px `line.hairline`。

### 4.11 悬停提示
延迟 500ms；`select.bg` 底、`select.fg` 文字（反色，Cursor featured 规则），`r.sm`，内距 4/8，“工具名 + keycap（反色版）”。

### 4.12 空状态（S3）
标题 `type.display`「开始一张截图」；三个入口卡片 168×116，`bg.card`，1px `line.strong`，`r.lg`，图标 24 `text.body`，标题 `type.button`，keycap；首个卡片“截图”左上 8px 橙色方块（品牌点）。下方拖放区 1px 虚线 `line.strong`，`r.lg`；拖入时边框 `brand` + `rgba(245,78,0,0.06)` 填充。

## 5. 交互状态总表

| 状态 | 规则 |
|---|---|
| Hover | 透明 → `bg.hover`；不变色、不放大 |
| Focus | 2px `brand` 外环（仅键盘） |
| Pressed | `bg.pressed` / `primary.pressed` |
| Selected | 工具：`select.bg` 反色；对象：双色选中框 + 手柄 |
| Disabled | `text.disabled`；不响应悬停 |
| Loading | 画布中央 20px 环形进度 `text.decor`，旧内容保持；状态栏 `pill.busy` |
| Empty | §4.12 |
| Error | Toast 错误图标 + `error` 文字 + 重试；输入框 `error` 边框 |
| Success | Toast `pill.done`；复制后窗口关闭（02 U07） |

## 6. 图标

- 线性图标，24 网格、1.5px 线宽、圆角端点；在 32 按钮中绘制 18px，颜色 `text.body`。
- 来源候选：Lucide（ISC），使用前复核许可（NOT_VERIFIED）。
- 映射：选择 `mouse-pointer-2`、箭头 `move-up-right`、框 `square`、文字 `type`、序号 自绘圆形数字、高亮 `highlighter`、遮挡 `shield`、马赛克 `grid-2x2` + 盾牌角标、裁剪 `crop`。

## 7. Rust 主题落地（草案）

```rust
// apps/desktop/src/ui/theme.rs（06 阶段创建；框架无关的 token 常量）
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

pub mod color {
    use super::Rgba;
    pub const BG_CANVAS:    Rgba = Rgba(0xf7, 0xf7, 0xf4, 0xff);
    pub const BG_WORKSPACE: Rgba = Rgba(0xec, 0xeb, 0xe5, 0xff);
    pub const BG_CARD:      Rgba = Rgba(0xff, 0xff, 0xff, 0xff);
    pub const LINE_STRONG:  Rgba = Rgba(0xcf, 0xcd, 0xc4, 0xff);
    pub const TEXT_INK:     Rgba = Rgba(0x26, 0x25, 0x1e, 0xff);
    pub const BRAND:        Rgba = Rgba(0xf5, 0x4e, 0x00, 0xff);
    pub const PRIMARY_BG:   Rgba = Rgba(0xd0, 0x42, 0x00, 0xff);
    pub const SECURE:       Rgba = Rgba(0x1f, 0x8a, 0x65, 0xff);
    // …其余 token 与 §2 表一一对应
}
pub mod size { pub const TOOLBAR_H: f32 = 52.0; pub const CONTROL: f32 = 32.0; /* … */ }
```

- 内容色（§2.3）会进入导出像素，属于产品语义，放在 `shareready-editor` 的标注颜色枚举；界面 token 只在 GUI crate。
- 框架为 Slint 1.18.1：token 以 `.slint` 中的 `global Theme` 为界面单一来源（已在 spike 中验证写法），Rust 侧只保留会进入导出像素的内容色。
- Slint 落地约束（来自 Gate 03 验证）：按钮内部文字节点设为不可访问，避免 Narrator 重复朗读；Slint Path 无原生虚线，遮挡对象虚线用分段绘制；画布拖拽中由 Slint 绘制轮廓，松手后由 Rust 合成不超过屏幕分辨率的显示预览。

## 8. 验收方法
- 每个组件在真实应用截图，与 [03_VISUAL_SAMPLE.html](03_VISUAL_SAMPLE.html) 并排对照（100% 与 150% DPI）。
- 新增或修改颜色时用 WCAG 公式重算对比度并更新 §2。
- 变更写入修订记录，版本号递增。

## 修订记录
- v0.1（2026-10-19）：Raycast + Linear 暗夜工作台。
- v1.0（2026-10-19）：Gate 04 冻结；补充 Slint 落地约束。
- v0.2（2026-10-19）：按用户“更有个性/品牌感”意见改为 Cursor 暖奶油编辑风；主按钮改用 `#d04200` 以满足白字 AA（`#f54e00` 仅 3.52:1）；次要文字 `#6b6960` 替代 Cursor muted（3.84:1）；新增状态胶囊、遮挡深绿身份、等宽数值。

---

## Gate 04 检查
- [x] Token、排版、密度、组件、交互状态、适配规则已定义
- [x] Gate 03 spike 确认 token 可在 Slint 实现（spike/slint）
- [x] **用户确认冻结基线**：2026-10-19，v1.0。
