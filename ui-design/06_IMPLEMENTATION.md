# 06 工程实现 — ShareReady 桌面版（Slint）

上游：G01 PRD · G02 UX · G03 Slint 1.18.1 · G04 DESIGN v1.0 · G05 高保真（均已由用户确认，2026-10-19）

## 批次计划

| 批次 | 关键路径 | 状态 |
|---|---|---|
| **B1** | 打开图片（文件 / 粘贴 / 命令行参数）→ 实心遮挡（画、选、移、八向调整、删除、撤销重做、方向键）→ 所见即所得画布 → 复制 / 另存新 PNG → 关闭保护 | **完成（机器验证，受限）** |
| B2 | 屏幕截图覆盖层（多显示器）+ 托盘 + 全局快捷键 + 独立 Toast 窗口（复制后关闭编辑窗，U07） | 待开始 |
| B3 | 标注（箭头/框/文字/序号/高亮）+ Rust 光栅化 + 安全马赛克 + 遮挡颜色 + 新金样本 | 待开始 |
| B4 | 裁剪、拖放导入、缩放平移完善、全键盘、关于对话框 | 待开始 |

## B1 设计要点

- **位置**：`apps/desktop`（包名 `shareready-desktop`，可执行文件 `shareready.exe`），**独立 workspace 与 Cargo.lock**（冻结的 V1 `e1-spike` 锁定 `wasm-bindgen =0.2.105`，与 Slint femtovg 依赖链要求的 `=0.2.123` 冲突；V1 退役后再并入根 workspace）；依赖 `shareready-core` / `shareready-editor` / `shareready-platform-windows`，以及锁定版本 `slint =1.18.1`、`rfd =0.15.4`、`arboard =3.6.1`（与 POC/spike 相同）。
- **单一编辑真值**：所有遮挡、历史、版本令牌都在 `shareready-editor::Editor`；GUI 只发意图、显示结果。
- **画布**：Rust 解码后生成**不超过 2560×1600 的显示预览**（Gate 03 结论），Slint `Image` 显示；遮挡对象、选中框、手柄、正在拖出的矩形由 Slint 叠加层按 Rust 计算的坐标绘制。实心遮挡在显示与导出中使用同一颜色 Core `COVER`，因此画布即成品。
- **命中测试与坐标**：Rust（`canvas.rs`）完成 屏幕↔源像素 变换与 16px 手柄命中。
- **交付**：复制 / 保存时在后台线程运行 `ReviewWork::run`（Core 生成并回读校验新 PNG）→ `accept_review` → `finish(review)` → `begin(Copy|Save)` → `bytes_for` → 平台写入 → `finish`。期间任何编辑都会让旧结果失效，提示“内容已变化，请重新复制”。
- **复制**：`decode_generated_png` 后经 `arboard` 写入 RGBA 图像。**保存**：`rfd` 另存对话框 → `platform_windows::save_new`（永不覆盖，失败不留半成品）。
- **关闭保护**：最后一次成功交付后有改动 → S5 对话框。

## B1 与设计的已知差异（如实记录）

| 差异 | 原因 | 何时解决 |
|---|---|---|
| 实心遮挡颜色为 Core `COVER #111827`，非 DESIGN `#1a1915` | 冻结的 V1 金样本依赖 `COVER`；改色需新 Core API 与新金样本 | B3 |
| 复制后不关闭窗口，提示显示在窗口内 | 没有托盘与独立 Toast 窗口时，关闭窗口即退出程序 | B2 |
| 无遮挡时不能复制 / 保存 | Core 拒绝导出无遮挡 PNG（V1 安全策略）；产品增加标注后需重新决策 | B3 前需用户决定 |
| 标注工具显示为禁用 | 尚未实现 | B3 |
| 未支持拖放导入 | Slint 1.18 文件拖放接入待验证 | B4 |
| 遮挡对象编辑态边框为 1px 实线 `secure`，非虚线 | Slint 无原生虚线，B1 简化 | B3 |
| 字体为 Noto Sans CJK SC + 系统 Consolas，非 Inter / JetBrains Mono | 字体下载需用户授权 | 待授权 |
| 无平移，100% 超出画布时固定在左上 | B1 范围 | B4 |

## B1 验收
- `cargo test --workspace --locked` 全部通过（含新增 desktop 单元测试）；5 张冻结金样本通过。
- 真实程序：命令行打开合成图 → 真实鼠标拖出遮挡、移动、调整 → `Ctrl+C` → 由另一进程（PowerShell）从系统剪贴板读回图像，逐像素核对遮挡区域为 `COVER`、其他区域与源一致。
- 真实截图与 05 高保真对照（H04/H06/H10/H12/H16 对应状态）。
- 独立代理审查。

## 证据（B1，2026-10-19，代理执行，非真人测试）

构建：`cargo build --release --locked --manifest-path apps/desktop/Cargo.toml`（本机使用 `CARGO_TARGET_DIR=spike/slint/target` 复用 Slint 编译缓存）。release `shareready.exe` SHA256 `1af336b6a0c0c915a30f0fcebba9936d91f783fd6b7cd22466c87feb82808cd7`。

| 检查 | 结果 |
|---|---|
| 桌面应用单元测试 `cargo test --locked`（apps/desktop） | **11 passed**：画布适应/命中（16px 半开手柄、手柄优先于选区）、指针意图（画、移、调整、撤销重做、微调、删除）、复核→交付写出 COVER 像素、编辑中复核作废、旧 worker 结果不能消耗当前复核、写入失败可重试、换图重置历史 |
| 根 workspace `cargo test --workspace --locked` | 72 passed（Core / Editor / platform 未改动） |
| 冻结金样本 `python tests/golden/verify_v1_baseline.py` | `SR_V2_00_FROZEN_GOLDEN_VERIFIED` |
| 端到端 `tests/desktop/e2e.ps1`（真实系统鼠标键盘） | 命令行打开合成截图 → 鼠标拖出遮挡 → 方向键 ×3 → Ctrl+C；**另一进程读回剪贴板**并逐像素比对：区域 (521,143)-(599,163) 1560 像素全为 COVER，其余 265040 像素与源一致，`CLIPBOARD_PIXELS_PASS`。已交付后关闭直接退出；有未交付编辑时关闭弹出 S5，Esc 取消后仍运行；空状态 Ctrl+V 粘贴打开 620×430 |
| 截图 | `tests/desktop/evidence/b1/01–07*.png`，与 05 的 H04/H06/H11/H16/H01 对照：布局、令牌、文案一致；已修复工具组居中、Toast 截断、空状态工具误选中 |
| 独立代理审查（feature-dev:code-reviewer） | 5 项发现全部修复：①“复制后关闭”标志在复制未真正开始时残留 → 仅在开始成功后设置，保存结束清除，Enter 需可交付；②旧 worker 结果可能消耗当前复核并卡住 → 结果携带 ReviewKey，非当前直接忽略（新增单测）；③对话框打断拖拽导致卡住 → 打开/选择对话框时取消拖拽，并对“正在拖动/正在处理”给出提示；④对话框打开时 Ctrl+O/V/S 仍生效、文件选择器可重复打开 → 加对话框与选择器重入保护；⑤Alt/Win 组合误触单键工具 → 忽略 |

**未验证 / 未运行（如实保留）**
- 另存为的真实系统对话框、`save_new` 端到端、同名拒绝（平台层已有单元测试，B1 未做 UI 端到端）。
- 端到端未覆盖：鼠标移动/八向调整/删除/重做、拖拽中交付、拖拽中 Esc、复制后关闭、替换确认对话框、画布外松开鼠标。
- 剪贴板读回经 WinForms `GetImage`，只比较 RGB，未检查 alpha。
- 系统 125%/150% DPI、Narrator 实际朗读、真人可用性。
