# 更新日志 / Changelog

## 0.5.1 — 2026-11-05

### 新增 / Added
- 截图诊断（默认关闭）：只有设置环境变量 `ANSHOT_DIAGNOSE_DIR` 时，安截才在该文件夹写 `anshot-diag.txt`，记录每次截图各阶段的耗时，以及显示器尺寸、缩放、HDR 状态、色深、显卡和 Windows 版本；不记录画面、窗口标题、路径或文字。配套的 `tools/diagnose/anshot-diagnose.ps1` 可在出问题的电脑上一键运行。/ Capture diagnostics, off by default: only with `ANSHOT_DIAGNOSE_DIR` set does Anshot write `anshot-diag.txt` there (timings of each capture step and display facts such as size, scaling, HDR, colour depth, adapter and Windows build; never pixels, window titles, paths or text). `tools/diagnose/anshot-diagnose.ps1` runs it on an affected machine.

### 修复 / Fixed
- 截图历史超过 200 张时（主要是“200 张”上限或设了保留天数时），张数上限和保留天数都没有完全执行：清理只看了历史窗口列出的最新 200 张。现在清理会检查文件夹里安截的全部历史图片，并且只认安截写下的文件名（不碰其他文件）。**升级后，过去本应删除却遗留下来的旧图片会被一次性删除**：超过保留天数的在下次启动时删除，超过张数的在下次存入历史时删除，范围以你当前的“截图历史”和“历史保留”设置为准。/ With more than 200 history images, neither the count limit nor the age limit was fully applied: clean-up only looked at the newest 200. It now covers every Anshot history image in the folder (nothing else). **After upgrading, old images that should already have been removed are deleted in one go:** those past the age limit at the next start, those past the count at the next save to history, as your current history settings say.
- 依赖审计：`pin-utils`（Slint 构建期依赖，不进入程序）被标为无人维护，暂无替代版本，已按规则记录忽略原因。/ Dependency audit: `pin-utils`, a build-time dependency of Slint not shipped in the program, is now unmaintained; the exception is recorded with its reason.

### 已知问题 / Known issues
- Windows 11 上有启动截图时短暂黑屏、显示闪烁的反馈，原因还在排查，本版没有修复。截图选择模式下选区以外会主动变暗，这是界面设计，不代表系统亮度被修改。/ Reports from Windows 11 of a brief black screen and flicker when a capture starts are still under investigation and not fixed in this version. Outside the selection the capture overlay deliberately dims the screen; the system brightness is not changed.

## 0.5.0 — 2026-11-02

### 新增 / Added
- 成品美化（Ctrl+Shift+B 或工具栏）：把截图放在背景上（8 组配色或从截图边缘自动取色），可调留白、圆角、阴影，按 1:1 / 4:3 / 16:9 / 3:4 / 9:16 出图（截图居中、不裁切），可加 macOS、Windows 或浏览器外框和自己输入的标题 / 地址文字。面板打开时画布显示成品预览。截图区域的像素与不带美化时完全相同（圆角切掉的角除外），遮挡不受影响；成品始终不透明。/ Beautify: put the capture on a background (8 palettes or colours taken from its edges) with padding, rounded corners and a shadow, at 1:1 / 4:3 / 16:9 / 3:4 / 9:16 (centred, never cut), with optional macOS, Windows or browser window chrome and your own title or address text. The canvas previews the result while the panel is open. Pixels of the capture are unchanged except under the rounded corners, covers included; the result is always opaque.
- 设置 → 常规 → 新截图：可按上次的样式自动美化（默认关）。外框文字不会被记住。/ Settings → General → New captures: optionally beautify with the last style (off by default). Window text is never remembered.

### 修复 / Fixed
- 依赖 pulldown-cmark 升到 0.13.6（RUSTSEC-2026-0414、RUSTSEC-2026-0433，只涉及安截不使用的 Markdown 维基链接选项）。/ pulldown-cmark 0.13.6 for two advisories about a Markdown option Anshot does not use.

## 0.4.0 — 未单独发布，并入 0.5.0 / not released on its own; shipped in 0.5.0

### 新增 / Added
- 分享前检查（Ctrl+Shift+K 或工具栏）：本机识别后列出疑似手机号、身份证号、银行卡号、邮箱、IP 地址和密钥，画布上标出位置；确认后才遮挡，一次撤销即可恢复。这是辅助检查：在合成测试图上约每十处漏一处，姓名和地址找不到。/ Check before sharing: lists what looks like phone, ID and card numbers, mail, IP addresses and keys, recognised locally; nothing changes until you apply, and one undo reverts it. An aid only: about one in ten synthetic values is missed, and names and addresses are not found.
- 复制 / 保存后自动检查（默认开，可在设置 → 隐私关闭）：不拦截；发现新内容时提示，点“查看”可遮挡并重新复制。/ A check after each copy and save (on by default; Settings → Privacy): never blocks; offers to cover and copy again.
- 同词全遮：选中一个遮挡，“遮住相同内容”找出图中其他相同文字。/ Cover the same text everywhere from one redaction.
- 替换代替打码：遮挡处可写占位文字或自己输入的文字（如“用户 A”）；占位文字不由原文生成。/ Replace with stand-in text instead of a plain cover; stand-ins are never derived from the original.
- 出图自检：有遮挡时核对遮挡下的文字在成品中是否还能被识别出来。/ After delivery, checks whether text under each cover can still be recognised in the result.

### 修复 / Fixed
- 关于窗口里的“检查更新”按钮在 0.2.0 / 0.3.0 中不起作用（对话框打开时命令被拦下），已修复。/ "Check for updates" in About did nothing in 0.2.0 and 0.3.0.

## 0.3.0 — 2026-10-27

### 新增 / Added
- 识别文字（离线 OCR）：选区工具条、编辑器（Ctrl+Shift+O）与钉图菜单；用 Windows 自带的文字识别，在本机完成、不联网、不下载；编辑器里识别的是导出图，被遮挡的内容识别不到；结果可编辑、可复制，不落盘。/ Text recognition (offline OCR) from the selection toolbar, the editor (Ctrl+Shift+O) and pins, using the OCR built into Windows: local, no network, no download; the editor reads the export render, so redacted text is not recognised; results are editable, copyable and never saved.
- 截图辅助：延时截图（3 / 5 / 10 秒，托盘或 `--delayed-capture`）、R 重复上次区域、选区“尺寸”（1:1 / 4:3 / 16:9 或精确宽高）、Shift 锁定正方形 / 比例、放大镜显示取色值并可 Shift+C 复制、可选截入鼠标指针。/ Capture aids: delayed capture, repeat the last area (R), exact size and ratios, Shift-locked square / ratio, colour readout with Shift+C, optional mouse pointer.
- 标注：聚光灯（S，其余部分变暗）、放大镜标注（Z，2 倍放大框，取的是遮挡后的画面）；Alt+滚轮调整选中的编号、线宽或字号。/ Annotations: spotlight (S) and magnifier callout (Z, 2x, sampled from the redacted picture); Alt+wheel adjusts the selected number, width or text size.
- 隐私设置：遮挡四周多盖 4 像素（默认开）、复制 / 保存未遮挡的图片时提醒、复制不进 Windows 剪贴板历史和云同步（默认开）、历史按天数清理、退出时清空。/ Privacy settings: redactions 4 px larger all round (default on), a warning when an image without redactions is copied or saved, copies kept out of clipboard history and the cloud clipboard (default on), history age limit, clear on exit.
- 英文界面：默认跟随 Windows 显示语言，可在设置里切换。/ English interface, following the Windows display language; switchable in Settings.

### 改进 / Improved
- 长截图：选区里只有一部分在滚动（侧栏、第二栏不动）时，只拼接滚动的那一栏；空白边距保留。/ Long screenshot: when only part of the area scrolls, only that pane is stitched; blank margins stay.
- 按快捷键后截图层立即获得键盘焦点（此前 Esc / Enter 要先点一下才生效）。/ The capture overlay gets keyboard focus right after the hotkey (Esc / Enter needed a click first).
- 截图时钉图与识别结果窗口暂时让开，不再被拍进去；倒计时、提示、长截图控制条不进入任何截图。/ Pins and the OCR window step aside while the screen is frozen; the countdown, notices and long-screenshot panel never appear in captures.
- 设置分为“常规 / 隐私”两页。/ Settings split into General and Privacy.
- GitHub 仓库改名为 `2023violet/Anshot`（旧地址自动跳转）；内部 crate、测试变量统一为 anshot / `ANSHOT_*`；新截图历史文件名前缀为 `anshot-`，仍读取 0.2.0 的 `sr-` 文件；移除从未发布的 0.1.x 的迁移代码；README 加入发布片片段并写明只支持 Windows。/ Repository renamed; internal names unified; new history files use the `anshot-` prefix and 0.2.0 files are still listed; 0.1.x migration code removed; README gains a film excerpt and states Windows-only support.

### 修复 / Fixed
- 图片复制时请求的“不进剪贴板历史”标记此前被 arboard 忽略；改为在同一次剪贴板会话里写入 PNG、CF_DIBV5 与标记。/ The clipboard-history opt-out was ignored for images by arboard; images are now written in one clipboard session with the documented flags.
- e2e 第 26 部分按旧前缀统计历史文件，改名后该检查实际未生效，已修正。/ e2e part 26 counted history files by the old prefix after the rename; fixed.
- 代理安全与代码审查（非人工审查）发现的问题已修复：剪贴板先写隐私标记、任何失败都清空剪贴板；截图失败时恢复让开的钉图与识别窗；手动截图取消正在进行的倒计时；精确尺寸不跳到另一块显示器；竖直 Shift 拖动也是正方形；识别文字同一时间只跑一个；历史天数在每次写入后也清理，“退出时清空”在启动时也执行；裁剪掉的遮挡不再算作“已遮挡”；小窗口里设置可滚动。/ Agent security and code review findings (not human review) fixed.


## 0.2.0 — 2026-10-21（首个公开版本 / first public release）

改名为 **安截 Anshot**（原临时代号 ShareReady）。/ Renamed to **Anshot (安截)**, formerly code-named ShareReady.

### 新增 / Added
- 截图选区工具条：直接复制 / 保存 / 钉图 / 长截图 / 编辑（Ctrl+C / Ctrl+S / Ctrl+T）。/ Selection toolbar: copy, save, pin, long screenshot or edit straight from the capture.
- 长截图：滚动内容，本机逐帧拼接，识别固定页头页脚。/ Long screenshot: scroll the content, frames are stitched locally, sticky headers and footers are recognised.
- 钉图到桌面：置顶、拖动、缩放、透明度、复制 / 保存。/ Pin to desktop: always on top, drag, zoom, opacity, copy and save.
- 截图历史（Ctrl+H）：保存编辑器复制或保存过的图片（与分享出去的一致，可能含未遮挡内容）；默认开启 50 张，可 20 / 200 或关闭；本机未加密存储。/ Capture history: the images the editor copied or saved, exactly as shared (may contain uncovered content); on by default (50), 20 / 200 or off; stored unencrypted locally.
- 标注：椭圆、直线、画笔；自定义颜色（HSV / HEX，最近使用）。/ Annotations: ellipse, line, pen; custom colour (HSV / HEX, recent colours).
- 安装程序：按当前用户安装、无需管理员、可卸载；便携 zip；SHA256SUMS 与 release-metadata.json。/ Per-user installer without admin rights, portable zip, checksums and release metadata.
- 关于：版本号与“检查更新”（只用浏览器打开发布页，程序不联网）。/ About: version and "check for updates" (opens the releases page in the browser; the app never connects).

### 改进 / Improved
- 放大镜在截图层打开时即跟随指针（R7）。/ The magnifier follows the pointer as soon as the overlay opens (R7).
- 弹出的提示不再抢走键盘焦点。/ Notifications no longer take keyboard focus.

### 安全 / Security
- 恶意 / 畸形图片语料与变异测试；JPEG 解码在截断的渐进式文件上卡死、在异常采样因子上 panic 的问题已在入口处拦截。/ Hostile-image corpus and mutation tests; a JPEG decoder hang (truncated progressive files) and panic (bad sampling factors) are now stopped at intake.
- 独立代理安全审查的 F1–F7 已修复：后续动作只作用于对应图片、钉图不入历史且保存前校验、历史列表与缩略图一致、文件读取有界并拒绝符号链接、长截图尺寸限制、页脚高度固定、开机启动项只在原 exe 不存在时改指向。/ Agent security review findings F1–F7 fixed.
- `cargo deny` 依赖审计（许可证、advisories、来源）。/ `cargo deny` dependency audit.

### 迁移 / Migration from 0.1.x
- 设置：若新目录 `%APPDATA%\Anshot` 没有设置，会读取 0.1.x 的 `%APPDATA%\ShareReady\settings.txt`（不修改旧文件）。/ Settings fall back to the 0.1.x file (read only).
- 0.1.x 写入的开机启动项 “ShareReady” 会被替换为 “Anshot”。/ A 0.1.x start-at-login entry is replaced.

### 已知限制 / Known limits
未签名；无自动更新；未做真人可用性测试；多显示器、125%/150% 系统缩放、Windows 11、讲述人未验证。/ Unsigned; no auto-update; no human usability test; multi-monitor, 125/150 % scaling, Windows 11 and Narrator not verified.

## 0.1.1 — 2026-10-20（本地构建 / local build）
代理产品评审 R1–R6：截图前隐藏编辑窗、人话错误提示、工具提示、Shift 约束、开机启动、可调选区与裁剪框。/ Agent product review fixes R1–R6.

## 0.1.0 — 2026-10-19（本地构建 / local build）
Slint 重写的首个完整版本：截图、标注、实心 / 安全马赛克遮挡、裁剪、复制 / 另存、托盘与全局快捷键、设置。/ First complete Slint rewrite.
