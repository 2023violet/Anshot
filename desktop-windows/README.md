# ShareReady V1.2 — Windows 桌面便携版

这是一个真正的 Windows 窗口程序（ShareReady.exe），不是 CMD 启动本地服务器。

- 原生窗口框架：.NET 8 WinForms + Microsoft Edge WebView2 1.0.4258.31。
- 图片像素处理：继续复用原有 Rust/WASM；没有把遮挡改成容易泄露原图的 CSS 模拟。
- Web 资源：从随应用分发的 `www/` 文件夹以内部虚拟 HTTPS 域映射加载，不需要 Python、PowerShell 服务或浏览器标签页。
- 运行时联网：宿主阻断非 `https://shareready.local/` 的 HTTP(S) 请求，网页本身仍启用 CSP。不存在图片上传或用户账户功能。
- 只允许访问 `www/` 中打包的编辑器资源；关闭新窗口/外站导航。
- 启动时为 WebView2 创建隔离的临时用户数据文件夹，关闭时尝试清理；**无法担保操作系统级彻底擦除**。
- Windows 端当前自包含 .NET Runtime，但需要电脑安装 **Microsoft Edge WebView2 Runtime**（Windows 11 一般具备）。

## 使用

1. 解压整个 `ShareReady-Desktop-Windows-x64` ZIP。
2. 在解压的文件夹内直接双击 `ShareReady.exe`。
3. 选择、拖放或粘贴 PNG/JPEG；手动框选需要遮挡的区域。
4. 在右侧选择区域，可输入 X/Y/宽/高四项原图像素坐标，支持撤销。
5. 检查 Rust 重新编码的成品 PNG，再复制或另存。

需 Windows 10/11 x64。若系统缺少 WebView2 Runtime，程序会提示安装。该程序目前没有代码签名、自动更新及 Windows 安装器；**不要误认为已经过外部安全认证**。

## 开发与测试

```powershell
dotnet publish desktop-windows/ShareReady.Desktop.csproj -c Release -r win-x64 --self-contained true
```

发布文件夹还需要一个 `www` 子目录，至少包含 `index.html`、`app.js`、`pkg/shareready_e1.js`、`pkg/shareready_e1_bg.wasm`。
CI 使用 **同一提交**重新编译 Rust/WASM、运行真实 Google Chrome 图片测试、再将生成的 JS/WASM 放入原生 Windows 包。

注意：CI 中可验证 .NET 编译及打包文件完整性，但不能把它当作 Windows 用户桌面上真实的 GUI、剪贴板和安装环境的全面验收。
