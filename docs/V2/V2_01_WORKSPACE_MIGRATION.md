# SR-V2-01｜Rust Workspace / Core / Editor 实际迁移记录

状态：ENGINEERING_INTEGRATION_PENDING_EXACT_SHA_CI（在最终 CI 核实前不得报告 PASS）。

## 已实现架构

单一根 Cargo Workspace：shareready-core、shareready-editor、shareready-e1 三成员。Core 从原始 E1 lib.rs 移入完全相同的 V1.3 解码、EXIF、区域裁剪、像素覆盖、预览与新 PNG 逻辑及 Rust 单元测试；E1 保留 CLI/WASM API 符号，改为 re-export 和 wasm-bindgen 薄层。

Editor 是真正新增的 Rust 领域库，包含文档代际 DocumentId、单调 Revision、操作 OperationId、像素闭合裁剪后半开区间、受限 1024 区域、128 历史、撤销重做、Review/Copy/Save 异步令牌与过期响应拒绝。用户改图、撤销或替换文档后，前一版 Review/Copy/Save 回调不能冒充当前操作完成。本阶段尚未接入 JS 编辑器或真实 OS clipboard；Editor 的完成状态只能由有实际系统操作证据的宿主确认。

依赖图：

    shareready-editor -> shareready-core -> image 0.25.8
    shareready-e1 -> shareready-core + wasm-bindgen 0.2.105

未引入第三方 GUI、Tokio、SQLite、HTTP 或 OCR。

## 锁文件和构建

权威 Cargo.lock 已迁入根目录，E1 子目录 Cargo.lock 暂作为不可依赖的历史快照保留，待最终减重阶段处理。所有 workspace 成员共享根 target 目录。CLI / 预览例子的真实位置为 target/release/...；WASM 为 target/wasm32-unknown-unknown/release/shareready_e1.wasm。既有 Python 安全清单和金样本检测指向新路径；工作流监听 Cargo.toml / Cargo.lock / crates/**。

## 验收判据

R01 cargo fetch --manifest-path Cargo.toml --locked；
R02 cargo test --workspace --locked --offline，覆盖 Core 旧单测及 Editor 新测试；
R03 cargo check --workspace --all-targets --locked --offline；
R04 V1 CLI、WASM 旧 JS ABI、Chrome 编辑器通过；
R05 V2-00 五份完整图片的冻结 RGBA SHA256 全部严格匹配；
R06 恶意资源输入检查、根 Cargo.lock 依赖盘点通过；
R07 Windows WinForms/WebView2 原生 Smoke / ZIP 构建通过；
R08 main 唯一，原 V1.3 web-app、desktop-windows、核心算法语义未删除，锁文件无测试漂移。

仅提交文档或看见新 Rust 源码不等于通过 R01-R08；所有结论须绑定最终 main HEAD GitHub Actions 实际结果。

## 下一阶段 SR-V2-02

继续移植选择、拖拽、八向手柄、区域移动与变更合并、viewport 几何、实际源图 bytes 的持有和异步结果发布。SR-V2-03 另行验证 egui/iced。当前 Rust Editor 尚不是完整可交付原生 GUI。Windows 人工 Explorer 拖放、跨程序剪贴板、多屏 DPI、Narrator 和安全独立审查均 NOT_VERIFIED，不得冒充已经验收。
