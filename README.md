# ShareReady（临时代号）— Windows 截图 · 标注 · 遮挡 · 分享

> **状态：0.1.0 本地构建，已完成 01–07 阶段，代理验收结论 ACCEPT_WITH_LIMITS。未签名、未发布，未做真人可用性测试或安全审计。**
> ShareReady 只是临时代号，已有同名产品，未做商标清查。

按 **Ctrl+Shift+A** 截图，加上箭头、文字和序号，遮住不该公开的内容，再一键复制或保存。**所有像素处理和导出都由 Rust 完成**：遮挡区域被替换成不含原图信息的像素，导出的是全新的 PNG，画布显示什么，导出就是什么。

## 功能
- **截图**：全局快捷键或托盘图标；每个显示器一个冻结覆盖层；拖拽选区、单击吸附窗口、Enter 截当前屏、放大镜。
- **导入**：打开文件、拖放、粘贴（PNG / JPEG，按 EXIF 校正方向，单文件 ≤ 25 MB，≤ 2400 万像素）。
- **标注**：箭头、框、多行文字（中文，可用输入法，Shift+Enter 换行，双击修改）、自动递增序号、高亮；6 色、3 档粗细、5 档字号、文字底色。
- **遮挡**：实心（黑 / 白），或**安全马赛克**（颜色只由位置决定，与原图无关）。不提供可还原的模糊或像素化。
- **编辑**：选择、移动、八向调整、撤销 / 重做、裁剪、缩放（Ctrl+滚轮）与平移（滚轮 / 空格拖动 / 中键）、Tab 循环选择、方向键微调、Ctrl+方向键调整尺寸。
- **交付**：Ctrl+C 复制（成功后编辑窗收起，弹出提示）；Ctrl+S 另存为新 PNG，永不覆盖已有文件（同名自动改为“名称 (2).png”）。
- **设置**（Ctrl+, 或托盘）：更换截图快捷键（4 种）、复制后收起或保留编辑窗、默认保存文件夹；只存本机 `%APPDATA%\ShareReady`。
- **本地运行**：无网络请求、无账号、无遥测（运行期实测 0 个网络端点）。

快捷键：V 选择 · A 箭头 · R 框 · T 文字 · N 序号 · H 高亮 · B 遮挡 · M 安全马赛克 · C 裁剪 · 1–6 颜色 · Del 删除 · Ctrl+Z / Y 撤销 / 重做 · Ctrl+0 / 1 适应 / 100% · Ctrl+, 设置。

## 构建与运行（Windows 10/11 x64，Rust 1.85+）
```powershell
cargo build --release --locked --manifest-path apps/desktop/Cargo.toml
apps/desktop/target/release/shareready.exe            # 也可带一个图片路径参数
pwsh -File scripts/package.ps1                          # 本地打包到 dist/（未签名，不上传）
```

## 架构（Rust 可信边界）
| 层 | 位置 | 职责 |
|---|---|---|
| Core | `crates/shareready-core` | 解码、EXIF、实心替换、PNG 编码与回读校验（V1 冻结行为） |
| Render | `crates/shareready-render` | 场景合成：遮挡 → 标注 → 裁剪，导出与画布预览共用 |
| Editor | `crates/shareready-editor` | 对象、样式、撤销历史、版本令牌（过期结果拒绝交付） |
| Platform | `crates/shareready-platform-windows` | 屏幕采集、显示器与窗口边界、全局快捷键、剪贴板读取、不覆盖保存 |
| App | `apps/desktop` | Slint 1.18.1 界面、托盘、截图覆盖层、控制器 |

界面只发送意图、显示 Rust 渲染的结果。设计与验收文档见 [`ui-design/`](ui-design/)：[PRD](ui-design/01_PRD.md) · [UX](ui-design/02_UX_ARCHITECTURE.md) · [视觉方向与框架选型](ui-design/03_VISUAL_DIRECTIONS.md) · [DESIGN v1.0](ui-design/04_DESIGN.md) · [高保真](ui-design/05_HIGH_FIDELITY/README.md) · [实现记录](ui-design/06_IMPLEMENTATION.md) · [验收报告](ui-design/07_UI_QA_REPORT.md) · [最终审查](ui-design/07_FINAL_REVIEW.md)。

## 验证
- `cargo test --workspace --locked`（根 workspace，85 项）和 `cargo test --locked --manifest-path apps/desktop/Cargo.toml`（27 项）。
- `python tests/golden/verify_v1_baseline.py`：5 张冻结的 V1 金样本；渲染金样本在 `crates/shareready-render`。
- `tests/desktop/e2e.ps1` / `perf.ps1`：用真实系统键鼠驱动应用，由另一进程读回剪贴板并逐像素比对。只使用合成图片，证据截图只截应用窗口与合成区域。
- 以上均为代理执行的机器验证。**真人计时、多显示器 / 真实系统 DPI（只做了应用内 125% 模拟）、Narrator 朗读、中文输入法组字、Windows 11 均未验证**，详见验收报告 §2 与 §6。

## 许可
项目代码 [MIT](LICENSE)。界面使用 Slint，按 *Royalty-free 2.0* 许可（“关于”对话框显示 AboutSlint）；字体 Noto Sans CJK SC 按 [OFL-1.1](assets/fonts/OFL-1.1.txt)。第三方清单：[THIRD_PARTY_LICENSES.md](apps/desktop/THIRD_PARTY_LICENSES.md)。

## 历史
V1.3（`web-app/`、`desktop-windows/`、`e1-spike/`，JS + C# WebView2 + Rust/WASM）和 V2-03 egui 样机（`poc/native-egui/`）作为冻结参考保留（快照 tag `pre-redesign-2026-10-19`），文档在 [`docs/`](docs/)。Stage 01 的用户研究（招募、真人测试）从未执行，仍为 NOT_VERIFIED。

**请勿在 issue 中附带真实的私人截图。**
