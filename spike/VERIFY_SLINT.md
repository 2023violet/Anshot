# Slint 1.18.1 条件验证（Gate 03 §5）

日期 2026-10-19　操作者：Claude 代理（自动化，经真实系统输入；非真人测试）
程序：`spike/slint-verify`（独立包，复用 `spike/slint/target` 构建目录）；驱动脚本：`spike/verify.ps1`；对照实验：`spike/ime_control.ps1`。
环境：Windows 10 19045，1920×1080，系统缩放 100%，GTX 1650（femtovg/OpenGL），微软拼音（zh-Hans-CN，HKL 0x08040804）。
原始日志：`spike/evidence/verify/*.txt`；截图：`spike/evidence/verify/*.png`。

## 复现
```bash
cd spike/slint-verify && CARGO_TARGET_DIR=../slint/target cargo build --release
```
```powershell
pwsh -File spike/verify.ps1 -Exe spike/slint/target/release/spike-slint-verify.exe -Log <txt> [-Updates 20] [-Interact] [-Scale 1.5] [-Backend winit-software] [-Shot <png>]
pwsh -File spike/ime_control.ps1
```

## 结果

| # | 条件 | 结果 | 证据 |
|---|---|---|---|
| 1 | 许可证 | **已决定**：用户接受 Royalty-free 2.0；`AboutSlint` 已在验证程序中渲染，UIA 可读 `#MadeWithSlint` | 截图 `femtovg-100-after.png` |
| 2 | 24MP 更新 | **MEASURED**：6000×4000 RGBA，每次“合成 + 拷贝到新 `SharedPixelBuffer` + `set_image`”中位 **57 ms**（p95 62，n=20）；到渲染通知器 `AfterRendering` 中位 **107 ms**（p95 114，max 139），femtovg 100%；150% 缩放下 104 / 116 / 144 ms。**这是 CPU 侧下限**：`AfterRendering` 在 GL 命令提交后、`present` 之前触发，不含 GPU 完成、缓冲交换与 vsync；其中约 57 ms 是合成本身。峰值工作集 631–651 MB、私有 584 MB（源图 96 MB + 合成缓冲 + GPU 纹理）。软件渲染：20 次更新在同一个计时回调中连续执行、中间不渲染，set 中位 58 ms 只含 CPU 合成 + 拷贝 + `set_image`，**不含软件光栅化**；帧完成时间未测（渲染通知器不支持）；峰值私有 195 MB | `femtovg-100-perf.txt`、`femtovg-150-perf.txt`、`software-100-perf.txt` |
| 3 | 拖拽与 16×16 命中区 | **MEASURED / PASS**（femtovg 100%、femtovg 150%、软件 100%）：移动 +30/+20 → `150 120 160 60`；在右下手柄中心偏移 6/6px（±4 可视之外、命中框之内；代码为 `<=8`，实际命中框是含边界的 17×17）拖 +25/+15 → `150 120 185 75`；左上手柄 +10/+10 → `160 130 175 65`；离手柄 14px 按下 → 未命中、不变（该点也在选区外，未测手柄与选区重叠时的优先级；8/9px 边界未测）。命中与状态在 Rust 中处理 | `*-interact.txt`、`femtovg-150-perf.txt` |
| 4 | 中文输入法 | **MEASURED / PASS**：微软拼音切换到中文模式后输入 `nihao␣`，`TextInput` 提交 **“你好”**（GPU 与软件渲染均通过）。对照：同样按键在记事本中切换后为 `你好`（阳性对照有效）。未切换时的 `nihao ` 结果来自脚本修改前的一次运行，输出未保存（UNSUPPORTED）。以 Shift 切换依赖输入法初始状态，测试较脆弱；预编辑显示、候选窗位置、Esc 取消、选候选词未测 | `*-interact.txt`、`ime_control.ps1` 输出 |
| 5 | Narrator | **部分**：UIA 树（Narrator 的数据来源）中 9 个工具按钮为 `Button` 且名称为中文（选择…裁剪）、可聚焦；输入框为 `Edit`，名称“标注文字”。每个按钮还暴露一个同名 `Text` 子节点，Narrator 可能重复朗读，产品中需设为不可访问。画布、选中框、遮挡对象无语义；Tab 遍历未测。**Narrator 实际朗读未测**（需真人） | `femtovg-100-interact.txt` 中 `UIA` 行 |
| 6 | 修复虚线/工具栏后重截、Inter/JetBrains Mono/粗体 | **NOT_RUN**：字体下载需用户授权；属视觉细节，不阻断框架选择 | — |
| 7 | 无 OpenGL 回退 | **MEASURED（模拟）**：`SLINT_BACKEND=winit-software` 正常启动、渲染、拖拽、输入法均通过；真实无 GPU 机器未测。软件渲染缩小 24MP 图时为近邻采样，细线丢失（截图 `software-100-after.png`） | `software-100-interact.txt` |
| — | 系统 125%/150% DPI | **NOT_VERIFIED**：只用 `SLINT_SCALE_FACTOR` 模拟；修改显示设置属系统设置，需用户操作 | — |

## 对产品设计的影响
- 24MP 全量重传约 0.1 秒/次：**编辑完成后**刷新可接受，**拖拽过程中**不可逐帧重合成。产品画布应：拖拽中由 Slint 绘制选中框/对象轮廓（本次已验证精度），松手后再由 Rust 合成；并由 Rust 生成不超过屏幕分辨率的显示预览（同时解决软件渲染近邻缩小的画质问题），全分辨率只用于导出与 100% 局部查看。
- 内存：显示预览化后可避免 GPU 持有 24MP 纹理；目标与 PRD 性能项在 07 阶段实测。
- Slint 默认带 `system-tray` 特性，可覆盖 02 U05 的托盘需求（HYPOTHESIS，未测）。

## 采集中的异常
- 一次软件渲染运行在脚本开始交互前收到两次非脚本的鼠标按下（疑似桌面上的真人操作），该次交互结果作废；两次异常按下在 (206,138)、(285,161)，把选区拉到 572×405，之后所有脚本拖拽都被识别为整体拖动。加入 `HARNESS interact_begin` 标记后重测，结果干净（`software-100-interact.txt`）。该次的性能数据保留（`software-100-perf.txt`），其中交互行不作为证据；它引用的 `software-100-after.png` 已被干净运行覆盖，与该日志不对应。

## 独立审查
第二轮代理审查见 [REVIEW.md](REVIEW.md)「第二轮：Slint 条件验证审查」：ACCEPT_WITH_LIMITS，可选定 Slint；以上更正均来自该审查。
