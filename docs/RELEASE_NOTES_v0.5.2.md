# 安截 Anshot 0.5.2（草稿，未发布）

**截图层画好才出现；HDR 提示。** 所有处理仍在本机完成，程序不联网。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.5.2-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可直接覆盖升级） |
| `Anshot-0.5.2-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值，再点“更多信息 → 仍要运行”。

## 本版内容
- **截图层画好才出现：** 先在后台把冻结的画面画好（最迟 300 毫秒），再显示到屏幕上，不会先露出一个还没画内容的窗口。
- **变暗渐入：** 选区以外的变暗用 120 毫秒渐入，不再一下子压暗。
- **HDR 提示：** 显示器开着 HDR 时，截图后提示一次，可选“不再提示”，也可在设置 → 常规关闭。
- **诊断对照组：** 诊断包新增 `-Groups`，用于在出问题的电脑上逐项排查（只在诊断开启时生效）。

## 已知问题与兼容性说明
- **HDR：** 在一台 Windows 11 24H2 + AMD Radeon 780M 电脑上，开着 HDR 截图会黑屏几秒、之后屏幕偏暗；关掉 HDR 后不再出现（用户实测）。具体机制还没确定，**本版没有消除这个问题**，只加了提示和诊断手段。
  - Win+Alt+B 可以快速开关 HDR；屏幕偏暗时按 Win+Ctrl+Shift+B 可恢复，不用重启。
  - HDR 屏上，截图颜色也可能偏淡（截屏接口只支持 8 位色）。
- 截图选择模式下，选区以外的画面会主动变暗，这是界面设计，不代表系统亮度被修改。
- 主要实机验证仍在 Windows 10、单显示器、100% 缩放；未做真人可用性测试；多显示器、系统缩放、讲述人未验证；未签名、无自动更新。

---

# Anshot (安截) 0.5.2 (draft, not released)

**The capture overlay appears only once drawn; an HDR notice.** Everything still happens locally; the app never connects to the network.

## What's in it
- **Overlay appears once drawn:** the frozen picture is drawn first (300 ms at the latest) and only then shown, so an empty window never reaches the screen.
- **Dimming fades in** over 120 ms instead of snapping on.
- **HDR notice:** after a capture, once per run, when a display is in HDR; "Don't show again", or off in Settings → General.
- **Diagnostic groups:** the kit gains `-Groups` for one-variable tests on an affected PC (only with diagnostics on).

## Known issues and compatibility
- **HDR:** on a Windows 11 24H2 PC with an AMD Radeon 780M, captures with HDR on went black for seconds and left the screen dim; with HDR off they did not (measured by the user). The mechanism is not known and **this version does not remove the problem**; it adds the notice and the diagnostics.
  - Win+Alt+B toggles HDR; Win+Ctrl+Shift+B restores a dim screen without a reboot.
  - On HDR displays captured colours can also look washed out (the capture API is 8-bit only).
- The dimming outside the selection is the interface; the system brightness is not changed.
- Tested mainly on Windows 10 with one monitor at 100% scaling; no human usability testing; multi-monitor, scaling and Narrator not verified; unsigned, no auto-update.
