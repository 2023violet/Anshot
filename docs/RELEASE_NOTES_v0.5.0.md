# 安截 Anshot 0.5.0

**成品美化：发出去的截图，直接是一张好看的图。** 背景、留白、圆角、阴影、出图比例和窗口外框都在本机由 Rust 合成；截图本身的像素不变，遮挡不受影响，程序不联网。0.5.0 包含尚未单独发布的 0.4.0（分享前检查），见 [0.4.0 发布说明](RELEASE_NOTES_v0.4.0.md)。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.5.0-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可在“应用和功能”卸载；可直接覆盖升级） |
| `Anshot-0.5.0-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值（`Get-FileHash <文件> -Algorithm SHA256`，与 `SHA256SUMS.txt` 一致），再点“更多信息 → 仍要运行”。

## 本版内容
- **成品美化**（Ctrl+Shift+B 或工具栏按钮）：
  - 背景：8 组配色，或从截图边缘自动取色；
  - 留白、圆角、阴影：无 / 小 / 中 / 大；
  - 比例：自由、1:1、4:3、16:9、3:4、9:16，截图居中、不裁切；
  - 外框：macOS、Windows 或浏览器样式，标题 / 地址文字由你输入，可留空。
- **所见即所得**：面板打开时画布显示成品预览，关掉开关就恢复原样；美化不占撤销步骤。
- **输出可信不变**：截图区域的像素与不带美化时完全相同（只有圆角切掉的四角不同），遮挡和标注不受影响；成品始终不透明，贴到任何软件都不会出现黑角。
- **检查只看截图**：复制 / 保存后的分享前检查只读成品里的截图区域，背景和外框文字不报。
- **设置**：常规 → 新截图，可按上次的样式自动美化（默认关）。外框文字不会被记住，也不写进设置。
- **修复**：pulldown-cmark 升到 0.13.6（两个安全公告只涉及安截不使用的 Markdown 维基链接选项）。

## 已知问题与兼容性说明（2026-11-05 补充）
- Anshot v0.5.0 已在 Windows 10、单显示器、100% 缩放环境完成主要实机验证。
- 目前收到 Windows 11 环境下启动截图时短暂黑屏、显示闪烁的反馈，问题正在排查，尚未确定根因。
- 截图选择模式下，选区以外的画面会主动变暗，这是当前界面设计，不代表系统亮度设置被修改。
- 如果退出截图模式后仍存在显示异常，建议先完全退出 Anshot，并通过 GitHub Issues 提供 Windows 版本、显示器缩放比例、HDR 状态及复现步骤。无需提供包含个人信息的屏幕截图。
- 0.5.1 修复了截图历史超过 200 张时的清理问题，并提供截图诊断包，建议升级。

## 已知限制
- 外框是简化的示意样式，不追求和真实系统一模一样。
- 成品超过 2400 万像素时会先缩小留白，仍放不下就提示并不美化（复制和保存照常）。
- 外框文字由你输入，分享前检查不会检查这段文字。
- 各聊天软件、社交平台对成品的二次压缩没有测过。
- 未做真人可用性测试。只在 Windows 10、单显示器、100% 缩放下实测；系统缩放、多显示器、HDR、Windows 11、讲述人未验证。
- 无自动更新；名称未做商标清查。

构建：提交 dc3cfc8（`release-metadata.json`）；之后的提交只改文档和测试证据。完整说明见 README 与 [CHANGELOG](../CHANGELOG.md)。

---

# Anshot (安截) 0.5.0

**Beautify: the capture you share is ready to look good.** Background, padding, rounded corners, shadow, output ratio and window chrome are composed locally in Rust. The capture's own pixels are unchanged, covers included, and the app never connects to the network. 0.5.0 includes 0.4.0 (check before sharing), which was not released on its own; see the [0.4.0 notes](RELEASE_NOTES_v0.4.0.md).

## Downloads
| File | What it is |
|---|---|
| `Anshot-0.5.0-win-x64-setup.exe` | Installer: per user, no admin rights, uninstall from "Apps & features"; upgrades in place |
| `Anshot-0.5.0-win-x64.zip` | Portable: unzip and run `Anshot.exe` |
| `SHA256SUMS.txt`, `release-metadata.json` | Checksums and build facts |

> **Unsigned:** Windows may show "Windows protected your PC" on first start. Check the checksum first (`Get-FileHash <file> -Algorithm SHA256`, compare with `SHA256SUMS.txt`), then choose "More info → Run anyway".

## What's in it
- **Beautify** (Ctrl+Shift+B or the toolbar button):
  - background: 8 palettes, or colours taken from the capture's edges;
  - padding, corners and shadow: none / small / medium / large;
  - ratio: free, 1:1, 4:3, 16:9, 3:4, 9:16, with the capture centred and never cut;
  - window chrome in macOS, Windows or browser style, with title or address text you type (optional).
- **What you see is what you share:** the canvas previews the result while the panel is open; switch it off to get the plain image back. Beautify takes no undo steps.
- **Output trust unchanged:** the capture's pixels are identical to the plain export except at the four rounded corners, covers and annotations included. The result is always opaque, so no app shows black corners.
- **Checks read only the capture:** the check after copying or saving reads the capture area, not the background or the chrome text.
- **Settings:** General → New captures can beautify with the last style (off by default). Window text is never remembered or written to the settings.
- **Fixed:** pulldown-cmark 0.13.6 (two advisories about a Markdown option Anshot does not use).

## Known issues and compatibility (added 2026-11-05)
- Anshot v0.5.0 has been tested mainly on Windows 10 with one monitor at 100% scaling.
- We have reports from Windows 11 of a brief black screen and flicker when a capture starts. The cause is still under investigation.
- In the capture overlay, everything outside the selection is deliberately dimmed; the system brightness is not changed.
- If the display still looks wrong after leaving the capture overlay, exit Anshot completely and open a GitHub issue with your Windows version, display scaling, HDR state and steps. Screenshots with personal information are not needed.
- 0.5.1 fixes history clean-up beyond 200 images and adds a capture diagnostic kit; upgrading is recommended.

## Known limits
- The window chrome is a simplified sketch, not a faithful copy of any system.
- When the result would pass 24 megapixels the padding shrinks first; if it still does not fit, beautify is refused with a notice (copying and saving still work).
- The check before sharing does not check the window text you type.
- Recompression by chat apps and social sites is not tested.
- No human usability testing. Tested only on Windows 10 with one monitor at 100% scaling; scaling, multi-monitor, HDR, Windows 11 and Narrator are not verified.
- No auto-update. The name has not been trademark-cleared.

Built from commit dc3cfc8 (`release-metadata.json`); later commits change only docs and test evidence. See the README and the [CHANGELOG](../CHANGELOG.md).
