# 更新日志 / Changelog

## 0.3.0 — 未发布 / unreleased

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
