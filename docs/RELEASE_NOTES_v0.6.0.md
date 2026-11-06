# 安截 Anshot 0.6.0

**步骤图：几张截图排成一张带编号和说明的图。** 所有处理仍在本机完成，程序不联网。本版包含没有单独发布的 0.5.2（截图层画好才出现、HDR 提示）。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.6.0-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可直接覆盖升级） |
| `Anshot-0.6.0-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值，再点“更多信息 → 仍要运行”。

## 本版内容
- **步骤图（Ctrl+Shift+J 或工具栏）：** 写操作说明、工单、向客服描述问题时，不用再到别的软件里拼图。
  - 加入步骤：截图工具条上的“加入步骤”（Ctrl+J）；编辑器里按 Ctrl+J 加入当前图（遮挡和标注已画进去）；历史记录里的“步骤”；步骤栏里的“从文件加入”。
  - 整理：调整顺序、给每一步写一行说明、编辑或删除某一步（删除可撤销）。
  - 排版：纵向、横向或网格；间距、背景（白 / 浅 / 深）、编号、说明、统一宽度可调；画布上显示预览。设置会记住排版，不记住说明文字。
  - “拼合并编辑”生成一张新图，在编辑器里照常标注、分享前检查、美化、复制和保存。拼合后步骤仍保留，可以回到步骤栏调整后重新拼合。
  - 编号和说明画在每张截图上方，不压住截图；不缩放时，每一步的像素原样保留；成品始终不透明。超过 2400 万像素时整体缩小，仍放不下就提示。
- **连拍：** 打开后，选好区域按 Enter 就加入步骤，不打开编辑器。从步骤栏或托盘开关；每次启动时为关。
- **步骤只在内存里：** 最多 20 步（合计不超过 1 亿像素），退出程序或“清空”后就没有了；说明文字不写进设置；历史只保存交付出去的拼合成品。没经过编辑器的截图在步骤栏标“未编辑”，提醒你拼合后在编辑器里遮挡和检查。
- **截图层画好才出现（来自 0.5.2）：** 先在后台把冻结的画面画好（最迟 300 毫秒）再显示，不会先露出一个还没画内容的窗口；选区以外的变暗用 120 毫秒渐入。
- **HDR 提示（来自 0.5.2）：** 显示器开着 HDR 时，截图后提示一次，可选“不再提示”，也可在设置 → 常规关闭。诊断包新增对照组 `-Groups`（只在诊断开启时生效）。

## 已知问题与兼容性说明
- **HDR：** 在一台 Windows 11 24H2 + AMD Radeon 780M 电脑上，开着 HDR 截图会黑屏几秒、之后屏幕偏暗；关掉 HDR 后不再出现（用户实测）。具体机制还没确定，**本版没有消除这个问题**；截图层画好才显示、变暗渐入这两项改动在那台电脑上的效果**未验证**。
  - Win+Alt+B 可以快速开关 HDR；屏幕偏暗时按 Win+Ctrl+Shift+B 可恢复，不用重启。
  - HDR 屏上，截图颜色也可能偏淡（截屏接口只支持 8 位色）。
- 步骤图打开“统一宽度”或超限缩小时，文字会变糊；默认不缩放。
- 连拍加入的是没有遮挡过的原始截图；拼合后请在编辑器里检查。
- 截图选择模式下，选区以外的画面会主动变暗，这是界面设计，不代表系统亮度被修改。
- 主要实机验证仍在 Windows 10、单显示器、100% 缩放；未做真人可用性测试；多显示器、系统缩放、讲述人未验证；未签名、无自动更新。

---

# Anshot (安截) 0.6.0

**Step image: several captures laid out as one image with numbers and captions.** Everything still happens locally; the app never connects to the network. This version includes 0.5.2, which was not released on its own (overlay appears once drawn, HDR notice).

## Download
| File | What it is |
|---|---|
| `Anshot-0.6.0-win-x64-setup.exe` | Installer (per user, no administrator rights; installs over the old version) |
| `Anshot-0.6.0-win-x64.zip` | Portable: unzip and run `Anshot.exe` |
| `SHA256SUMS.txt`, `release-metadata.json` | Checksums and build facts |

> **Unsigned:** Windows may show "Windows protected your PC" on first run. Check the checksum first, then "More info → Run anyway".

## What's in it
- **Step image (Ctrl+Shift+J or the toolbar):** for how-tos, tickets and support requests, without joining images in another app.
  - Add steps: "Add step" (Ctrl+J) on the capture toolbar; Ctrl+J in the editor adds the current image with its covers and annotations; "Step" on a history item; "Add from file" in the panel.
  - Arrange: reorder, give each step a one-line caption, edit or delete a step (delete can be undone).
  - Layout: vertical, horizontal or grid; spacing, background (white / light / dark), numbers, captions and same width, previewed on the canvas. The layout is remembered; captions are not.
  - "Join and edit" opens a new image in the editor, ready to annotate, check, beautify, copy and save. The steps stay, so you can go back, change them and join again.
  - Numbers and captions sit above each capture, never over it; unscaled steps keep their pixels exactly; the result is always opaque. Past 24 MP it is scaled down, or refused with a message.
- **Burst:** Enter on a selection adds it as a step without opening the editor. Switch it from the panel or the tray; it is off at every start.
- **Steps live in memory only:** 20 at most (100 MP in all), gone on exit or "Clear"; captions are never saved in the settings; history keeps only the delivered result. Captures that never went through the editor are marked "Unedited" as a reminder to cover and check after joining.
- **Overlay appears once drawn (from 0.5.2):** the frozen picture is drawn first (300 ms at the latest) and only then shown; the dimming outside the selection fades in over 120 ms.
- **HDR notice (from 0.5.2):** after a capture, once per run, when a display is in HDR; "Don't show again", or off in Settings → General. The diagnostic kit gains `-Groups` (only with diagnostics on).

## Known issues and compatibility
- **HDR:** on a Windows 11 24H2 PC with an AMD Radeon 780M, captures with HDR on went black for seconds and left the screen dim; with HDR off they did not (measured by the user). The mechanism is not known and **this version does not remove the problem**; whether the drawn-first overlay and the fading dimming help on that PC is **not verified**.
  - Win+Alt+B toggles HDR; Win+Ctrl+Shift+B restores a dim screen without a reboot.
  - On HDR displays captured colours can also look washed out (the capture API is 8-bit only).
- "Same width", and scaling past the size limit, make text in a step image blurrier; nothing is scaled by default.
- Burst adds raw captures that have not been covered; check the joined image in the editor.
- The dimming outside the selection is the interface; the system brightness is not changed.
- Tested mainly on Windows 10 with one monitor at 100% scaling; no human usability testing; multi-monitor, scaling and Narrator not verified; unsigned, no auto-update.
