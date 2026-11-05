# 安截 Anshot 0.5.1

**维护版：修复截图历史的清理范围，加入可选的截图诊断。** 所有处理仍在本机完成，程序不联网。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.5.1-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可在“应用和功能”卸载；可直接覆盖升级） |
| `Anshot-0.5.1-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值（`Get-FileHash <文件> -Algorithm SHA256`，与 `SHA256SUMS.txt` 一致），再点“更多信息 → 仍要运行”。

## 本版内容
- **修复：截图历史超过 200 张时没有完全按设置清理。** 以前清理只看最新的 200 张，“截图历史”张数和“历史保留”天数都可能落空。现在会检查文件夹里安截的全部历史图片，不碰其他文件。
  - **升级后注意：** 过去本该删除却遗留下来的旧图片会被一次性删除：超过保留天数的在下次启动时删除，超过张数的在下次存入历史时删除，范围以你当前的设置为准。想保留的话，请在升级前从历史中另存。主要影响把上限设为 200 张、或设了保留天数，并且历史曾超过 200 张的情况。
- **新增：截图诊断（默认关闭）。** 只有设置了环境变量 `ANSHOT_DIAGNOSE_DIR` 时，安截才写诊断文件，内容只有耗时和显示器信息，不含画面、窗口标题、路径或文字。仓库里的 `tools/diagnose/anshot-diagnose.ps1` 可以在出问题的电脑上一键运行，详见 `tools/diagnose/README.md`。

## 已知问题与兼容性说明
- 安截 0.5.x 已在 Windows 10、单显示器、100% 缩放环境完成主要实机验证。
- 目前收到 Windows 11 环境下启动截图时短暂黑屏、显示闪烁的反馈。问题正在排查，尚未确定根因，**本版没有修复**。
- 截图选择模式下，选区以外的画面会主动变暗，这是当前界面设计，不代表系统亮度设置被修改。
- 如果退出截图模式后仍有显示异常，请先完全退出安截（托盘图标右键 → 退出），再通过 GitHub Issues 提供 Windows 版本、显示器缩放比例、HDR 状态和复现步骤；可以附上诊断包生成的两个文本文件。无需提供含个人信息的屏幕截图。
- 其余限制同 0.5.0：未做真人可用性测试；系统缩放、多显示器、HDR、Windows 11、讲述人未验证；未签名、无自动更新；名称未做商标清查。

完整说明见 README 与 [CHANGELOG](../CHANGELOG.md)。

---

# Anshot (安截) 0.5.1

**Maintenance release: history clean-up now covers every image, and optional capture diagnostics.** Everything still happens locally; the app never connects to the network.

## Downloads
| File | What it is |
|---|---|
| `Anshot-0.5.1-win-x64-setup.exe` | Installer: per user, no admin rights, uninstall from "Apps & features"; upgrades in place |
| `Anshot-0.5.1-win-x64.zip` | Portable: unzip and run `Anshot.exe` |
| `SHA256SUMS.txt`, `release-metadata.json` | Checksums and build facts |

> **Unsigned:** Windows may show "Windows protected your PC" on first start. Check the checksum first (`Get-FileHash <file> -Algorithm SHA256`, compare with `SHA256SUMS.txt`), then choose "More info → Run anyway".

## What's in it
- **Fixed: with more than 200 history images, clean-up did not follow your settings.** It only looked at the newest 200, so neither the count nor the age limit was fully applied. It now covers every Anshot history image in the folder, and nothing else.
  - **After upgrading:** old images that should already have been removed are deleted in one go: those past the age limit at the next start, those past the count at the next save to history, as your current settings say. Save any you want to keep before upgrading. This mainly affects a 200-image limit or an age limit, with more than 200 images ever stored.
- **New: capture diagnostics, off by default.** Only with the environment variable `ANSHOT_DIAGNOSE_DIR` set does Anshot write a diagnostics file: timings and display facts only, never pixels, window titles, paths or text. `tools/diagnose/anshot-diagnose.ps1` in the repository runs it on an affected machine; see `tools/diagnose/README.md`.

## Known issues and compatibility
- Anshot 0.5.x has been tested mainly on Windows 10 with one monitor at 100% scaling.
- We have reports from Windows 11 of a brief black screen and flicker when a capture starts. The cause is still under investigation and **this version does not fix it**.
- In the capture overlay, everything outside the selection is deliberately dimmed. This is the interface; the system brightness is not changed.
- If the display still looks wrong after leaving the capture overlay, exit Anshot completely (tray icon → Exit), then open a GitHub issue with your Windows version, display scaling, HDR state and steps; you can attach the two text files from the diagnostic kit. Screenshots with personal information are not needed.
- Other limits are as in 0.5.0: no human usability testing; scaling, multi-monitor, HDR, Windows 11 and Narrator are not verified; unsigned, no auto-update; the name has not been trademark-cleared.

See the README and the [CHANGELOG](../CHANGELOG.md).
