# ADR-0001（草案）｜Rust-first 产品载体与图像处理边界

> **SR-ADR-0001 · 2026-10-14 · PROPOSED / NOT ACCEPTED**  
> Architecture Decision Record 候选。不能将“Rust 项目语言”混同为“浏览器应用必须全部用 Rust”；首发框架须先经过实际输入/导出/体验验证。**本轮未写 Rust 代码、未安装任何新依赖。**

## 背景

用户确定以 Rust 打造 ShareReady。产品当前是假设中的“分享前处理”，目标是下载门槛低、本地处理、安全导出；尚未证明它比成熟工具更好用。技术路线应该服从首发使用场景，而不是为 Rust 选择一个过重的 GUI 平台。

## 不变需求

1. 核心几何坐标运算、遮挡区域裁剪、像素变更、最终 PNG 编码及可测试的输入校验逻辑，由 Rust 库拥有；绝不能仅在 CSS 叠层上“看起来变黑”。
2. 用户图片不提交远端服务；所有默认保存或复制来自完成遮挡与重编码的**新输出数据**。禁止直接复用原 JPEG/PNG 文件缓冲区以冒充处理结果。
3. 提供输出图与敏感区域真值的机器对照测试；图像元数据、缩放、RGBA、边界、异常大文件及 clipboard 内容单独验证。
4. 不因 UI 框架选择扩大产品功能范围、引入云端账号、AI OCR 或服务器。
5. 正式发布前必须完成威胁模型、依赖/许可证审核、具体平台行为检查和针对真实截图**风险说明**；不得承诺绝对匿名。

## 候选方案

| 方案 | 能力与权衡 | 决策前验证 |
|---|---|---|
| A — Rust core + Leptos CSR / WASM + Trunk | 浏览器即用；一套 Rust UI 和可测试核心；静态站可免费托管。代价：WASM 下载/初始化、剪贴板权限、Canvas/浏览器安全模型。 | 输入 PNG/JPEG、批量区域遮挡、PNG 导出、剪贴板写入，测首屏与大图内存/失败路径 |
| B — Rust core + Tauri 2 桌面 | 使用 Web UI 但系统窗口/文件/剪贴板更好集成，Rust 核心可复用。代价：打包签名、OS 权限、桌面跨平台测试。 | 从系统截屏到导出的完整路径与安装成本，Windows/macOS 实机 |
| C — Rust core + egui/eframe 或 Slint | 原生/混合桌面体验，有利于 Rust 主导 UI；跨平台编辑画布、输入法、触摸和打包复杂度不同。 | 坐标稳定性、画布控制、DPI 缩放和复制/保存集成的实际原型 |

## 建议（未批准）

**候选 A 作为第一项技术 SPIKE**：在 A1 用户需求通过并取得相应工程许可之后，以最小功能测 Rust/WASM + 静态站与剪贴板。不在现阶段锁死 Leptos 依赖版本，也不抢跑创建 Cargo workspace。

### 需要记录的实验指标

- (t_{init}) 页面可开始交互的时间；设备、网络条件和 WASM 包体积。
- 单张 1080p/4K 截图到输出的处理时间及最大内存估算；异常输入时不能导致无界分配。
- 图片遮挡像素矩阵中的边缘 + 旋转/EXIF 方向处理是否严格映射到输出。
- 处理前后 PNG 解码数据与元数据块差异；剪贴板写入后再次粘贴的是真正的成品。
- 主要浏览器 Chrome/Edge 桌面及待选择移动设备的权限/回退矩阵。

这些指标目前全部 `NOT_MEASURED`。数值门槛应根据实际用户任务确定，而不是假定 WASM 一定快。

## 最小代码层次（仅概念，不是代码）

```text
core (Rust native + wasm target)
  geometry -> redaction -> encode -> invariants/tests
ui (candidate Leptos / Tauri / native)
  input bridge -> working copy/state -> actual-output preview -> clipboard/download
platform adapter
  browser Clipboard/File API OR OS equivalent
```

**避免过度架构**：只有实际测试证明需要跨端时才拆更多 crate，避免抽象单次使用的接口或引入 async runtime。

## 参考（以官方文档为事实边界）

- Leptos CSR: https://book.leptos.dev/getting_started/index.html
- Leptos CSR static deployment: https://book.leptos.dev/deployment/csr.html
- Web clipboard: https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/write
- Clipboard MIME: https://developer.mozilla.org/en-US/docs/Web/API/ClipboardItem
- Tauri 2: https://v2.tauri.app/
- egui: https://github.com/emilk/egui
- Slint: https://docs.slint.dev/latest/docs/slint/

## 通过/拒绝规则

- 若主要用户要求桌面截图后无需离开原工作流，应重新比较 B/C，而非固守 A。
- 若 Web 能完成任务，但剪贴板限制在目标平台构成主要障碍，优先缩小平台声明或提供下载回退，再讨论桌面端。
- 如果 A1（需求优势）失败，则 **CANCEL TECH SPIKE**，即使 Rust 路线技术上可实现。
- 是否合并本 ADR、安装依赖和启动工程，均须经对应 Gate B 和工程阶段审批。

**状态：PROPOSED / RUST_CORE_REQUIRED / UI_HOST_NOT_FROZEN。**
