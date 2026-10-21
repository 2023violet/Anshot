# 更新日志 / Changelog

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
