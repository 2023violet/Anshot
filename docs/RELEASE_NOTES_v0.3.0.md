# 安截 Anshot 0.3.0

**截图辅助、离线识别文字、隐私辅助、英文界面。** 所有像素处理仍在本机由 Rust 完成，程序不联网。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.3.0-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可在“应用和功能”卸载；可直接覆盖升级 0.2.0） |
| `Anshot-0.3.0-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值（`Get-FileHash <文件> -Algorithm SHA256`，与 `SHA256SUMS.txt` 一致），再点“更多信息 → 仍要运行”。

## 本版内容
- **识别文字（离线 OCR）**：选区工具条、编辑器（Ctrl+Shift+O）、钉图菜单。用 Windows 自带的文字识别，本机完成、不联网、不下载。编辑器里识别的是导出图，被遮挡的内容识别不到。结果可编辑、可复制，不落盘。
- **截图辅助**：延时截图（3 / 5 / 10 秒）、R 重复上次区域、选区“尺寸”（1:1 / 4:3 / 16:9 或精确宽高）、Shift 锁定正方形 / 比例、放大镜显示颜色值（Shift+C 复制）、可选截入鼠标指针。
- **标注**：聚光灯（S）、放大镜标注（Z，取的是遮挡后的画面）；Alt+滚轮调整选中的编号、线宽或字号。
- **隐私设置**：遮挡四周多盖 4 像素（默认开）；复制 / 保存未遮挡的图片时提醒；复制的图片不进 Windows 剪贴板历史和云同步（默认开）；历史按天数清理；退出时清空历史。
- **英文界面**：默认跟随 Windows 显示语言，可在设置里切换。
- **改进**：长截图只拼接真正滚动的那一栏；按快捷键后截图层立即能用键盘；截图时钉图和识别结果窗口暂时让开，倒计时、提示、长截图控制条不会被截进去。

## 已知限制
- 未做真人可用性测试。
- 只在 Windows 10、单显示器上实测；多显示器、HDR、Windows 11、讲述人未验证（系统缩放的复测情况见 README）。
- 识别文字需要 Windows 里装有对应语言的 OCR 组件；准确率依赖系统组件，小字号或复杂背景会出错。
- 遮挡外扩能减少文字边缘外泄，但不能隐藏被遮内容的长度。
- 其他程序设了“不允许截图”的窗口，在截图里会是空白。
- 无自动更新；名称未做商标清查。

完整说明见 README 与 [CHANGELOG](../CHANGELOG.md)。

---

# Anshot (安截) 0.3.0

**Capture aids, offline text recognition, privacy aids and an English interface.** All pixel work still happens locally in Rust; the app never connects to the network.

## Downloads
| File | What it is |
|---|---|
| `Anshot-0.3.0-win-x64-setup.exe` | Installer: per user, no admin rights, uninstall from "Apps & features"; upgrades 0.2.0 in place |
| `Anshot-0.3.0-win-x64.zip` | Portable: unzip and run `Anshot.exe` |
| `SHA256SUMS.txt`, `release-metadata.json` | Checksums and build facts |

> **Unsigned:** Windows may show "Windows protected your PC" on first start. Check the checksum first (`Get-FileHash <file> -Algorithm SHA256`, compare with `SHA256SUMS.txt`), then choose "More info → Run anyway".

## What's in it
- **Text recognition (offline OCR)** from the selection toolbar, the editor (Ctrl+Shift+O) and pins. It uses the OCR built into Windows: local, no network, no download. In the editor it reads the export render, so redacted text is not recognised. Results are editable and copyable and are never saved.
- **Capture aids:** delayed capture (3 / 5 / 10 s), repeat the last area (R), exact size and ratios (1:1 / 4:3 / 16:9), Shift-locked square or ratio, a colour readout in the magnifier (Shift+C copies it), and an optional mouse pointer.
- **Annotations:** spotlight (S) and a magnifier callout (Z, sampled from the redacted picture). Alt+wheel adjusts the selected number, line width or text size.
- **Privacy settings:**
  - redactions 4 px larger on every side (on by default);
  - a warning when an image without redactions is copied or saved;
  - copied images kept out of Windows clipboard history and the cloud clipboard (on by default);
  - a history age limit, and clearing the history on exit.
- **English interface** that follows the Windows display language; switchable in Settings.
- **Improvements:**
  - long screenshots stitch only the pane that actually scrolls;
  - the capture overlay takes keyboard input right after the hotkey;
  - pins and the OCR window step aside during a capture;
  - the countdown, notices and long-screenshot panel never appear in captures.

## Known limits
- No human usability testing.
- Tested only on Windows 10 with one monitor. Multi-monitor, HDR, Windows 11 and Narrator are not verified; see the README for display-scaling results.
- Text recognition needs the Windows OCR component for the language. Accuracy depends on that component, and small text or busy backgrounds cause errors.
- Padding redactions reduces leaks at the text edges, but it cannot hide the length of the covered text.
- Windows of other programs that forbid capture appear blank in screenshots.
- No auto-update. The name has not been trademark-cleared.

See the README and the [CHANGELOG](../CHANGELOG.md).
