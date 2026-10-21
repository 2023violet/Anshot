# 安截 Anshot 0.2.0

**首个公开版本。** Windows 截图 · 标注 · 遮挡 · 分享——遮住的地方，导出里真的没有了。所有像素处理在本机由 Rust 完成，程序不联网。

## 下载
| 文件 | 说明 |
|---|---|
| `Anshot-0.2.0-win-x64-setup.exe` | 安装版（当前用户，无需管理员；可在“应用和功能”卸载） |
| `Anshot-0.2.0-win-x64.zip` | 便携版，解压运行 `Anshot.exe` |
| `SHA256SUMS.txt`、`release-metadata.json` | 校验值与构建信息 |

> **未签名提示**：首次运行 Windows 可能显示“已保护你的电脑”。请先核对校验值（`Get-FileHash <文件> -Algorithm SHA256`，与 `SHA256SUMS.txt` 一致），再点“更多信息 → 仍要运行”。

## 本版内容
- 截图选区工具条（复制 / 保存 / 钉图 / 长截图 / 编辑）、长截图拼接、钉图到桌面、截图历史（保存复制或保存过的图片，与分享出去的一致；默认开启、本机未加密存储，可关闭或清空）。
- 椭圆、直线、画笔、自定义颜色；Shift 约束；可调选区与裁剪框。
- 安全马赛克与实心遮挡：遮挡区域导出时不含原图任何信息；导出为不带元数据的新 PNG；永不覆盖已有文件。
- 安装程序、开机启动（可选）、关于页“检查更新”（只打开本页面）。
- 安全：恶意图片语料与变异测试，拦截了 JPEG 解码的一个卡死与一个 panic；代理安全审查发现的问题均已修复。

## 已知限制
未做真人可用性测试；只在 Windows 10、单显示器、100% 缩放下实测（多显示器、125%/150%、Windows 11、讲述人未验证）；无自动更新；名称未做商标清查。完整说明见 README 与 [CHANGELOG](../CHANGELOG.md)。

---

# Anshot (安截) 0.2.0

**First public release.** Screenshot, annotate, redact and share on Windows — what you cover is really gone from the export. All pixel work happens locally in Rust; the app never connects to the network.

## Downloads
| File | What it is |
|---|---|
| `Anshot-0.2.0-win-x64-setup.exe` | Installer: per user, no admin rights, uninstall from "Apps & features" |
| `Anshot-0.2.0-win-x64.zip` | Portable: unzip and run `Anshot.exe` |
| `SHA256SUMS.txt`, `release-metadata.json` | Checksums and build facts |

> **Unsigned:** Windows may show "Windows protected your PC" on first start. Check the checksum first (`Get-FileHash <file> -Algorithm SHA256`, compare with `SHA256SUMS.txt`), then choose "More info → Run anyway".

## What's in it
- Capture and pins:
  - a selection toolbar to copy, save, pin, take a long screenshot or edit;
  - scroll-and-stitch long screenshots;
  - pin to desktop;
  - a capture history of the images you copied or saved, exactly as shared. It is on by default, stored unencrypted locally, and can be turned off or cleared.
- Ellipse, line, pen and a custom colour picker. Shift constraints. Adjustable selection and crop box.
- Safe mosaic and solid redaction:
  - covered areas carry nothing of the original;
  - exports are fresh PNGs without metadata;
  - existing files are never overwritten.
- An installer, optional start at login, and About → check for updates (it only opens this page).
- Security: a hostile-image corpus and mutation tests caught a JPEG decoder hang and panic, now stopped at intake. All agent security-review findings are fixed.

## Known limits
- No human usability testing.
- Tested only on Windows 10 with one monitor at 100% scaling. Multi-monitor, 125/150% scaling, Windows 11 and Narrator are not verified.
- No auto-update.
- The name has not been trademark-cleared.

See the README and the [CHANGELOG](../CHANGELOG.md).
