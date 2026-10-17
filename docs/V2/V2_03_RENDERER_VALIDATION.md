# SR-V2-03B｜egui Windows OpenGL 与 wgpu/DX12 可用性对照

状态：STRICT_WGPU_LOCK_PINNED / RENDERER_RUNTIME_RECHECK_PENDING / PHYSICAL_WINDOWS_QA_NOT_RUN。

## 技术结论与边界

已真实验证的早期 OpenGL smoke 在 GitHub windows-latest 虚拟机失败，错误为 egui_glow requires opengl 2.0+。这证明**托管测试机的 OpenGL 环境不满足条件**，不证明 egui 在正常 Windows 设备上失败。

根据 [eframe 0.36.2 NativeOptions](https://docs.rs/eframe/0.36.2/eframe/struct.NativeOptions.html) 和 [egui-wgpu WgpuSetup](https://docs.rs/egui-wgpu/0.36.2/egui_wgpu/enum.WgpuSetup.html)，同一款 egui 样机可切换 Renderer::Glow / Renderer::Wgpu。WGPU_BACKEND=dx12 限制 Windows Direct3D 12，不依赖系统 OpenGL 2.0。RFD/Arboard 和 Rust Core/Editor 均继续复用。

### 真实初始对照与测试工具错误

- 功能提交 `24e5a56f4243ca098a407ee58600658c36a0bba4`，Windows Actions `#39009655418`：混合 glow+wgpu Rust EXE 编译、Core/Editor 真实图片 PNG 自检通过。
- OpenGL 探针如实返回 1，报缺少 OpenGL 2.0。
- wgpu_dx12 和 wgpu_auto 首轮探针均因 PowerShell `Get-Content -Raw` 对空输出产生 `$null`，随后调用 `.Contains()`/`.Trim()` 触发异常，**其 GUI 能力尚未测得**。首轮 `0/3` 不可解读为 wgpu 0/2 真正运行失败。
- `Cargo.lock` 已由 Actions 合入唯一 main（`6279c29bc06d8e8cedf94d0e0f221faf24573f09`）。此后 workflow 恢复 `permissions: contents: read`，仅允许 `--locked` 严格构建。
- 本轮修复探针确保空日志读取返回空字符串、输出真实 stderr/exit code、机器 JSON，并由独立 pwsh 进程传播非零退出码。**如果 0/3，窗口步骤应明确失败但仅作为图形环境专项阻断，不掩盖 Rust 编译状态。**

## 图形后端验证流程

| 路径 | 启动形式 | 需要的实际证据 |
|---|---|---|
| glow | `--renderer glow --smoke-window` | 多帧渲染窗口关闭标记；否则报真正 GPU 错误 |
| wgpu/DX12 | `$env:WGPU_BACKEND="dx12"; .\shareready-native-egui-poc.exe --renderer wgpu --smoke-window` | adapter backend=Dx12、device_type、窗口多帧 PASS |
| wgpu 自动 | 清除 WGPU_BACKEND；`--renderer wgpu --smoke-window` | 记录实际选择何种 backend、适配器类型、窗口多帧 PASS |

可以在 Windows 10/11 本地直接运行 `poc/native-egui/Run-Renderer-Probes.ps1`，将三个路径的退出码、设备、帧标记和 stderr 保存在 `poc/native-egui/evidence/renderer-matrix.json`；它只处理合成测试图片，不访问网络，也不上传信息。日常图形窗口直接运行无参数 EXE，默认优先 wgpu。

### 判定与下一阶段

1. DX12 runner smoke 通过：仅确立 `EGUI_WGPU_DX12_RUNNER_BOOTSTRAP_PASS`，如果是 WARP/软件适配器，不得宣称真实独显性能已通过。
2. DX12 runner smoke 失败：详细记录失败性质，核对是否是 CI VM 的 DX12/显示会话限制；没有真实 Windows GPU 与多屏人工证据，不能武断判 egui 失败。
3. 必须进行 G01–G10：四角像素、8 手柄、100–200% DPI、双屏、24MP 性能、Explorer 拖放、跨应用复制、保存取消、Narrator/中文输入、真实系统网络/许可证检查。
4. 如果有真实设备证据表明 egui 某关键 P0 项无法满足，必须先在 egui 内进行一次**有界修复与复验**。仍 FAIL 再启动 iced 同题 POC。
5. 现有 Windows C#/WebView2 V1.3 是正式产品，不移除也不发布 egui 为成品。

所有 CI 仅验证其源提交对应的结果；独立 G01–G10 人工签署缺失一律标 NOT_VERIFIED。
