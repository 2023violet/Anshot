# Gate 03 原生框架 spike 证据

日期 2026-10-19　基线 HEAD `1fbd8e8` + 本目录（未提交前）　操作者：Claude 代理（机器测量，非真人评审）
目标：用 egui / iced / Slint 分别渲染 [`ui-design/03_VISUAL_SAMPLE.html`](../ui-design/03_VISUAL_SAMPLE.html) 的主工作台骨架，按 04_DESIGN v0.2 token 对照。

## 环境
- Windows 10 19045，1920×1080，系统缩放 100%；rustc 1.98.1。
- 版本：eframe/egui **0.36.2**（wgpu）、iced **0.14.0**（canvas+image，默认 wgpu）、Slint **1.18.1**（默认 winit + femtovg）。
- 每个 spike 是独立 Cargo 包（`[workspace]` 空表），有自己的 `Cargo.lock`，不进入产品 workspace。
- 共享内容：`assets/shot.png`（`make_shot.py` 生成的合成聊天截图，620×430）。

## 复现命令
```bash
python spike/make_shot.py
cargo build --release --manifest-path spike/egui/Cargo.toml
cargo build --release --manifest-path spike/iced/Cargo.toml
cargo build --release --manifest-path spike/slint/Cargo.toml
```
```powershell
pwsh -File spike/capture.ps1 -Exe spike/<fw>/target/release/spike-<fw>.exe -Out spike/evidence/<fw>-100.png
pwsh -File spike/capture.ps1 -Exe spike/<fw>/target/release/spike-<fw>.exe -Out spike/evidence/<fw>-150.png -Scale 1.5
```
`capture.ps1` 把窗口移到屏幕原点，用 `PrintWindow(PW_RENDERFULLCONTENT)` 截客户区。150% 通过 `SPIKE_SCALE`（egui `set_zoom_factor`、iced `scale_factor`）或 `SLINT_SCALE_FACTOR` 模拟，**不是**系统 DPI 设置。

## 结果（MEASURED）

| 项 | egui 0.36.2 | iced 0.14.0 | Slint 1.18.1 |
|---|---|---|---|
| 构建 | 通过（无原始日志，UNSUPPORTED） | 通过（同左） | 通过（同左） |
| 有效源码行（.rs/.slint，非空） | 264 | 239 | 232 |
| release exe | 28.4 MB | 29.1 MB | 29.3 MB |
| 依赖 crate 数（Cargo.lock，含非 Windows 目标，偏高） | 301 | 468 | 558 |
| 空闲 5s 工作集 / 私有内存 | 185 / 411 MB | 175 / 238 MB | 80 / 35 MB |
| 100% 截图 | [egui-100](evidence/egui-100.png) | [iced-100](evidence/iced-100.png) | [slint-100](evidence/slint-100.png) |
| 150% 截图 | [egui-150](evidence/egui-150.png)（窗口不变，内容缩放） | [iced-150](evidence/iced-150.png)（窗口随比例放大） | [slint-150](evidence/slint-150.png)（同 iced） |
| 对比图 | [compare.png](evidence/compare.png) | | |

exe 体积三者接近，主要来自内嵌的 16 MB Noto Sans CJK SC。

## 观察（MEASURED = 截图可见；HYPOTHESIS = 未测）

| 维度 | egui | iced | Slint |
|---|---|---|---|
| 还原设计稿的方式 | 全部手工 painter 绘制；默认控件外观与设计差距大 | 控件 + 样式闭包，token 直接映射 | 声明式 `.slint` + `global` token，最接近 DESIGN.md 的写法 |
| 文字垂直居中（Noto CJK + Consolas） | **偏差明显**：中文偏下、数字偏上（MEASURED）；用 `FontTweak` 修正为 HYPOTHESIS | 按钮内文字偏上（MEASURED）；用 container 居中修正为 HYPOTHESIS | 居中正确（MEASURED） |
| 1px 发丝线、选中框、8×8 手柄 @100% | 精确（MEASURED） | 精确（MEASURED） | 精确（MEASURED） |
| 虚线 | 原生 `dashed_line` | 原生 `LineDash` | **无原生虚线**，需拼接（MEASURED；spike 中拼接长度写错，虚线越过马赛克框，属实现错误） |
| 150% 发丝线（审查逐像素取样） | **模糊**：1px 线变成两行半强度抗锯齿；窗口不随缩放变化（zoom 模式） | 对齐为 1 个全强度物理像素，最清晰 | 取整为 2 个全强度像素，清晰但偏粗；窗口超出屏幕时 PrintWindow 读不到 GL 表面外区域，移到原点后完整 |
| 画布自由绘制 | 最强（即时模式） | 强（canvas） | 较弱（Path/Rectangle 元素）；但 PRD U04 规定画布显示 Rust 合成图像，GUI 只画选中框/手柄，影响有限（HYPOTHESIS） |
| 无障碍 | 本 spike 关闭了默认特性，**未接入** AccessKit；eframe 可开启，但只覆盖真实控件，手绘控件无语义 | iced 0.14.0 / iced_winit / iced_widget / iced_core 中均**无 AccessKit**（DOCUMENTED，审查复核） | 默认开启 `accessibility`（accesskit 0.24 / accesskit_windows 在锁文件中）（DOCUMENTED） |
| 中文 IME | NOT_VERIFIED | NOT_VERIFIED | NOT_VERIFIED |
| 指针拖拽 / 24MP 纹理更新 | NOT_VERIFIED（POC 有历史证据） | NOT_VERIFIED | NOT_VERIFIED |
| 许可证 | MIT OR Apache-2.0 | MIT | GPL-3.0 **或** Slint Royalty-free 2.0（桌面免费，须在“关于”中显示 `AboutSlint` 或在下载页放署名徽标；非 OSI 开源许可）或商业 |
| 字重 | 只能按字体文件切换 | 同左 | 同左（三者都只加载了 Regular，粗体未测） |

## 限制
- 125% 未测（验收要求包含 125%）。Slint 截图中工具组居中而非紧随品牌字标，属 spike 布局错误。
- 内存数据无原始日志，但已由独立审查复现（185/411、175/239、80/35 MB）。
- Inter 与 JetBrains Mono 未下载（需单独授权的文件下载），spike 用 Noto Sans CJK SC（含拉丁）与系统 Consolas 代替；字距与 OpenType 特性未测。
- 150% 为框架缩放模拟；物理 125%/150% 显示器与混合 DPI 未测。
- 只测了静态渲染与悬停；拖拽、键盘焦点遍历、IME、Narrator、大图性能均未测。
- 所有判断由实现代理做出；独立审查记录见 `REVIEW.md`。
