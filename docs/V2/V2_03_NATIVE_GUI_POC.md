# SR-V2-03｜Rust 原生 Windows GUI 样机与技术验收

状态：WINDOWS_BINARY_AND_DOMAIN_TEST_PASS / NATIVE_GUI_RUNTIME_BLOCKED_OPENGL / G01-G10_ACCEPTANCE_PENDING（不可提前宣布 GO_EGUI）。

## 工程方案

新增独立目录 poc/native-egui ，它不是主 Cargo Workspace 成员。独立 Cargo.lock，严格使用已通过的 shareready-core 和 shareready-editor 两个 Rust 库，避免新 GUI 依赖污染 V1.3 的依赖图。固定 eframe 0.36.2（glow/AccessKit），rfd 0.15.4（原生文件对话框），arboard 3.6.1（系统图片剪贴板）。

样机实现真实 Rust 窗口、画布、鼠标框选、移动、八方向 Resize、遮挡选择、Undo/Redo、源图缩放、Explorer 拖放、文件打开、Rust Core 新 PNG 真值复核、复制图片和新文件保存。

画布展示使用 CPU 降采样后的最大 2048px 纹理；完整 PNG 导出只通过 Rust Editor/Core 原像素和真实图片源字节。UI 上的遮挡预览不得被截屏导出。保存 POC 使用 create_new 拒绝覆盖现有文件。Clipboard 在 Windows 经 arboard 输出的是 DIB/Bitmap 兼容图像，不保证存放原始 PNG 文件字节。App 不包含云同步、账号或遥测。

## 真实测试判定

| ID | 必须验证 | 当前状态 |
|---|---|---|
| G01 | 原生鼠标在 T04 四角进行 Fit/100/200% 绘制并以导出像素复核 | NOT_RUN_HUMAN |
| G02 | 原生八向手柄、键盘、1px、Undo/Redo | RUST_UNIT_VERIFIED / HUMAN_NOT_RUN |
| G03 | 100/125/150/200% DPI、多屏与窗体搬移 | NOT_RUN_HUMAN |
| G04 | 真正 24MP GPU 渲染、峰值内存和帧时 | NOT_RUN_MEASURED |
| G05 | Windows Explorer 真实拖放、原生对话框与取消 | NOT_RUN_HUMAN |
| G06 | Copy 回贴 Paint 和第二个程序，RGBA 一致 | NOT_RUN_HUMAN |
| G07 | Save 新文件、只读、取消、既有路径和重试 | NOT_RUN_HUMAN |
| G08 | 中文输入法、键盘任务、Narrator 语义 | NOT_RUN_HUMAN |
| G09 | 文档修改/异步结果过期令牌 | CORE_EDITOR_TESTED / GUI_HUMAN_NOT_RUN |
| G10 | Windows EXE、锁文件、许可/网络/离线验证 | CI_PENDING / NETWORK_AUDIT_NOT_RUN |

--self-test 是不启动 GUI 的真实 Rust Core/Editor PNG 测试；--smoke-window 是 GitHub Windows runner 尝试窗口生命周期，不能替代真实用户输入、系统对话框及 DPI 验收。所有操作必须绑定 SHA、Windows/设备/GPU/DPI、输出哈希和实际观测结果。无设备或实测不充分时只能报告 POC_ENGINEERING_READY / NATIVE_ACCEPTANCE_PENDING，不得冻结 egui 为终局方案。若 egui 关键 P0 无法修复，再同题评估 iced。

## 运行

Windows 安装 stable Rust、MSVC Build Tools；首先确认独立 Cargo.lock 已被 CI 写入 main，之后执行 cargo build --release --locked --manifest-path poc/native-egui/Cargo.toml。可执行文件位于 poc/native-egui/target/release/shareready-native-egui-poc.exe。打开默认 T04 合成图后直接拖出遮挡矩形，拖动框内部移动，框边缘/四角调整。Review PNG 后允许 Save/Copy。

正式用户版本仍为 Windows C# + WebView2 V1.3；本 POC 不授权移除该产品，不代表发行与独立安全认证。

## 二次原生 CI 事实证据（第 1 次成功编译）

- 修订提交 26122c79f0f4445874fccfb6cf2de8164cb36ff9，Actions Run #39006084786 整体 SUCCESS，但其中的 GUI Smoke 为 continue-on-error/FAIL，不得写成全部 GUI PASS。
- Windows 原生 EXE Release 编译成功，T04 内置合成图片 Rust Core/Editor 的 PNG 和版本自测 SR_V2_03_NATIVE_POC_SELFTEST_PASS。
- EXE SHA256: 4560d2e9e88bab5129a3f26a6de8cedc05e4e7f859365f9661de86f08b1d6ccd。
- Windows GUI 启动尝试输出：OpenGL(PainterError("egui_glow requires opengl 2.0+. "))。因此 Windows runner 图形环境不足，G01-G08 真实桌面证据仍 NOT_VERIFIED；不应臆测正常 Windows 主机一定失败。
- 隔离 Cargo.lock 已由 GitHub Actions 以非强推方式写到唯一 main，SHA 6159c18c14b33cc49b4355d0eb451f2a0820d74b。现在起 CI 只读并严格使用 --locked；不可再将自动生成的未锁版本当成最终通过。
- 下一正式判断：在具备 Windows 10/11 GUI 会话及真实 GPU/DPI 的设备运行 G01–G10；如 glow 不适合目标设备，再有边界地验证 wgpu（DX12）后端，而不是直接宣布更换 iced 或反向宣称 egui 已通过。
