# ShareReady V1 — 本地图片隐私遮挡编辑器

开箱即可使用的静态网页工具：Rust/WASM 本地解码与像素重编码，无登录、无上传接口。

## 运行

下载 GitHub Actions `ShareReady V1 local editor` 的最新 **成功**运行中
`shareready-v1-local-app-and-chrome-evidence` 工件，解压出
`ShareReady-V1` 文件夹。该文件夹已经包含 JavaScript 和编译好的 WebAssembly。

**Windows 10/11：** 解压后双击 `Start-ShareReady.cmd`。它通过 Windows 自带的 PowerShell
只在 `127.0.0.1` 开一个临时端口，自动尝试打开浏览器，**不需要 Python 或 Rust**。使用期间请保持启动窗口打开；关闭窗口即关闭本地服务。
首次从互联网下载的脚本可能遇到 Windows 的安全提示，请先核对 ZIP 来源和文件内容；本工具目前不是签名安装程序。

**macOS / Linux / 手动启动：** 在文件夹内启动一个本地 HTTP 服务（需要 Python 3）：

```bash
cd ShareReady-V1
python -m http.server 8000 --bind 127.0.0.1
```

然后在 Chrome/Edge 浏览器打开 **http://127.0.0.1:8000/**。

不要直接双击 `index.html`：`file://` 不能可靠加载 WASM，Clipboard 也需要可信上下文。静态资源均在本地文件夹，不需要登录。

## 可用功能

- 拖放、粘贴（Ctrl+V / Cmd+V）或选择任意本地静态 PNG/JPEG；不会限制为四份合成演示图片。
- Rust 校验最大 25 MiB 输入和 24 MP 解码像素，将 JPEG EXIF 方向统一后生成新的 PNG 编辑预览。
- 手动绘制多个实心遮挡、选中移动、缩放、键盘微调、删除、撤销/重做。
- 画布适合窗口、缩放与平移，源图像素坐标不会因显示比例改变。
- 点击 `100% 原图` 可按真实源像素比例查看，`0` 适应画布、`1` 100%；清空全部遮挡有确认，并支持撤销。
- “检查成品”是真正 Rust 编码后的独立 PNG 解码；可保存 PNG 或尝试复制 PNG 到剪贴板。
- 如果 Clipboard 被浏览器拒绝，请直接下载 PNG。下载不会回退为未经遮挡的原图。
- 只覆盖你手动选中的区域；**分享前必须仔细检查结果**。不含 OCR/AI 自动检测。

## 限制与隐私

- 单张 PNG/JPEG；暂不支持 PDF、SVG、视频、GIF 动画、批量处理。
- 全部像素数据仅在打开此工具的浏览器运行环境内使用，不设计服务端上传；
  当前 HTML CSP 禁止外站数据请求。不要把该静态编辑器放入第三方脚本或不可信页面中。
- 已在 Chrome 上用机器合成图片自动化测试，**不代表所有图片、操作系统、
  浏览器及高风险法律/医疗信息均得到安全认证**。
- 若未通过上传图片，就不会离开本地。工具不会替你识别任何未框选的隐私区域。

## 构建（开发者）

从 ShareReady 仓库根目录：

```bash
cargo test --manifest-path e1-spike/Cargo.toml --locked
rustup target add wasm32-unknown-unknown
cargo build --manifest-path e1-spike/Cargo.toml --locked --release --target wasm32-unknown-unknown --features web
cargo install wasm-bindgen-cli --version 0.2.105 --locked
wasm-bindgen --target web --out-name shareready_e1 --out-dir web-app/pkg e1-spike/target/wasm32-unknown-unknown/release/shareready_e1.wasm
cd web-app
python -m http.server 8000 --bind 127.0.0.1
```

若已有其他服务占用 8000 端口，改用任意空闲本地端口。

MIT/其他许可证以仓库实际 LICENSE 为准，未在这里额外宣称新的许可证。
