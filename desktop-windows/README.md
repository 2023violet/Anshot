# ShareReady V1.3-01 — Windows 原生运行体验

解压便携包，直接双击 `ShareReady.exe`，使用 Windows 原生窗口处理图片。保留原有 Rust/WASM 新 PNG 像素重编码，不依赖 Python/浏览器标签页/HTTP 本地服务器。

**Windows 用户工作流：** 选择 PNG/JPEG（系统文件选择器）→ 拖入或 Ctrl+V 粘贴（WebView2 文件事件）→ 画出实心遮挡→成品复核→选择保存路径并确认或复制 PNG。关闭原生窗口即结束本次任务。

本阶段新增：
- 使用 WebView2 内置选择图片和操作系统拖放入口，不需要把本地图像传给云服务；
- 下载 PNG 时由 Windows `SaveFileDialog` 选择保存位置，支持取消与覆盖确认，拒绝非本地 PNG 下载；网页状态与真实保存结果同步，不再把“点击下载”误报成保存成功；
- 阻止任何未经许可的对外 HTTP(S) 请求、新窗口、外站导航和无手势读取系统剪贴板，使用每次启动隔离的 WebView2 浏览器数据目录；
- 窗口尺寸遵循 Windows DPI；网页使用图像自身的像素坐标，避免显示缩放引起遮挡输出漂移；
- 缺少 WebView2 Runtime 和丢失资源时给出启动错误说明。

**测试边界：** CI 使用 Windows 原生 `ShareReady.exe --smoke-native` 打开 WebView2 并执行由合成 PNG 驱动的文件输入、DOM 拖放、DOM 粘贴、Rust 遮挡、成品下载和剪贴板成功/明确失败检测。该自动化不是物理 Windows 文件选择器手动点击、系统资源管理器真实拖拽或普通用户机器多屏幕 DPI 迁移的全面实机验收。

**需要环境：** Windows 10/11 x64，已安装 Microsoft Edge WebView2 Runtime。当前为未签名便携软件，非安装器。检查来源并完整解压，不运行来历不明的文件。
