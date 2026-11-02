# 安截 Anshot 0.4.0

**分享前把关：发出去之前，帮你找出可能的隐私内容、遮好、再核对一遍。** 识别和检查都在本机完成，程序不联网；识别出的文字只在内存中。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.4.0-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可在“应用和功能”卸载；可直接覆盖升级 0.3.0） |
| `Anshot-0.4.0-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值（`Get-FileHash <文件> -Algorithm SHA256`，与 `SHA256SUMS.txt` 一致），再点“更多信息 → 仍要运行”。

## 本版内容
- **分享前检查**（Ctrl+Shift+K 或工具栏按钮）：列出疑似手机号、身份证号、银行卡号、邮箱、IP 地址和密钥，画布上用方框标出，方框只在界面里，不会进入成品。默认全选，可逐项取消；点“遮挡所选”后才改图，一次撤销即可恢复。
- **复制 / 保存后自动检查**（默认开，设置 → 隐私可关）：图先交付，不拦截；发现新内容时提示，点“查看”可遮挡并重新复制。
- **同词全遮**：选中一个遮挡，点“遮住相同内容”，图中其他相同文字一起遮住。适合姓名这类规则找不到的内容。
- **替换代替打码**：遮挡处可以写占位文字（如 `138 XXXX 0001`、`user-a@example.com`、“已隐藏”），或自己输入（如“用户 A”）。占位文字不由原文生成，也不保留部分原文。
- **出图自检**：有遮挡时，交付后再核对遮挡下的文字在成品里是否还能被识别出来，例如文字标注盖在了遮挡上。
- **修复**：关于窗口里的“检查更新”在 0.2.0 / 0.3.0 中不起作用。

## 已知限制
- **这是辅助检查，不保证找全。** 在合成测试图上约每十处漏一处（手机号、IP、密钥和 12–14 px 小字更容易漏）；真实截图可能更差。姓名、地址、跨行内容、人脸、手写和图片里的文字查不到。
- 出图自检只说明 Windows 自带的识别器读不出，不能证明无法还原。
- 替换用内置字体，和原字体不同；渐变或图片背景上能看出色块。
- 检查需要 Windows 里装有对应语言的 OCR 组件；没有时按钮会给出安装说明，自动检查静默跳过。
- 未做真人可用性测试。只在 Windows 10、单显示器、100% 缩放下实测；系统缩放、多显示器、HDR、Windows 11、讲述人未验证。
- 无自动更新；名称未做商标清查。

完整说明见 README 与 [CHANGELOG](../CHANGELOG.md)。

---

# Anshot (安截) 0.4.0

**A check before sharing: before an image leaves, Anshot helps you find what might be private, cover it, and check the result.** Recognition and checking happen on this computer; the app never connects to the network, and recognised text stays in memory.

## Downloads
| File | What it is |
|---|---|
| `Anshot-0.4.0-win-x64-setup.exe` | Installer: per user, no admin rights, uninstall from "Apps & features"; upgrades 0.3.0 in place |
| `Anshot-0.4.0-win-x64.zip` | Portable: unzip and run `Anshot.exe` |
| `SHA256SUMS.txt`, `release-metadata.json` | Checksums and build facts |

> **Unsigned:** Windows may show "Windows protected your PC" on first start. Check the checksum first (`Get-FileHash <file> -Algorithm SHA256`, compare with `SHA256SUMS.txt`), then choose "More info → Run anyway".

## What's in it
- **Check before sharing** (Ctrl+Shift+K or the toolbar button): lists what looks like phone, ID and card numbers, mail addresses, IP addresses and keys, with boxes on the canvas that never reach the result. Everything is ticked; untick what you want to keep. Nothing changes until you press "Cover selected", and one undo reverts it.
- **A check after each copy and save** (on by default; Settings → Privacy): the image is delivered first and nothing is blocked. If something new turns up, "Review" lets you cover it and copy again.
- **Cover same text:** select a cover and every other copy of that text in the picture is covered too. Useful for names, which the rules cannot find.
- **Stand-in text instead of a plain cover,** such as `138 XXXX 0001`, `user-a@example.com` or "Hidden", or your own words ("User A"). Stand-ins are never derived from the original and keep none of it.
- **A check of the result:** when there are covers, Anshot checks after delivery whether the text under them can still be recognised, for example when a text annotation sits on top of a cover.
- **Fixed:** "Check for updates" in About did nothing in 0.2.0 and 0.3.0.

## Known limits
- **An aid, not a guarantee.** On synthetic test images about one value in ten is missed (phone numbers, IP addresses, keys and 12–14 px text more often), and real screenshots may do worse. Names, addresses, text across lines, faces, handwriting and text inside pictures are not found.
- The check of the result only shows that the OCR built into Windows cannot read the text; it does not prove the text cannot be recovered.
- Stand-in text uses the built-in font, not the original one, and shows as a patch on gradients or pictures.
- Checking needs the Windows OCR component for the language. Without it the button explains how to add it, and the automatic check is skipped quietly.
- No human usability testing. Tested only on Windows 10 with one monitor at 100% scaling; scaling, multi-monitor, HDR, Windows 11 and Narrator are not verified.
- No auto-update. The name has not been trademark-cleared.

See the README and the [CHANGELOG](../CHANGELOG.md).
