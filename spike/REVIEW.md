# Gate 03 原生框架 spike 独立审查

日期 2026-10-19　审查对象：`spike/README.md`、`spike/{egui,iced,slint}` 源码、`spike/evidence/*.png`、`spike/capture.ps1`

## 审查范围与身份

- 审查者：**独立子代理（Claude，非真人）**，未参与 spike 实现；本审查不等于真人视觉 / 可用性 / 无障碍评审。
- 只读核对；除本文件外未改动任何文件，未执行改变 git 状态的命令。
- 方法：逐图查看 7 张截图；用 System.Drawing 对截图逐像素取样（工具栏底部发丝线、截图外框、选中框）；
  重测 exe 体积、`Cargo.lock` 中 `name = ` 行数、非空源码行、空闲内存；读 cargo registry 中的 crate 元数据与许可证文本。
- 对照：`ui-design/04_DESIGN.md` §2–4、`03_VISUAL_SAMPLE.html`、`03_VISUAL_DIRECTIONS.md` §4 验收项。

## 核实结果

### 指标表

| 声明 | 结论 | 依据 |
|---|---|---|
| 源码行 264 / 239 / 232 | CONFIRMED（小注） | 重算一致；Slint 232 = app.slint 226 + main.rs 6，未计 build.rs 3 行，影响可忽略 |
| exe 28.4 / 29.1 / 29.3 MB | CONFIRMED | 29 761 536 / 30 542 336 / 30 761 472 B（MiB） |
| 依赖 crate 301 / 468 / 558 | CONFIRMED（需注明口径） | `Cargo.lock` 计数包含 macOS/Linux/iOS 等非 Windows 目标 crate（如 Slint 锁含 accesskit_macos/atspi），不等于 Windows 实际编译数 |
| 空闲内存 185/411、175/238、80/35 MB | CONFIRMED | 本审查重跑 6 s 后：185/411、175/239、80/35 MB；README 未附原始日志，建议补存 |
| 体积主要来自 16 MB Noto CJK | CONFIRMED | 字体文件 16 437 364 B，三者都 `include`/`import` 同一文件 |
| 构建情况 | UNSUPPORTED（无构建日志） | 源码与 exe 存在，但“首次通过 / 2 处 API 修正”无记录可查 |

### 观察表

| 声明 | 结论 | 依据 |
|---|---|---|
| egui 文字垂直偏差明显 | CONFIRMED | egui-100：「删除」低于 Del keycap，输入框数字偏上，「保存」与 keycap 基线错位 |
| “需逐字体 FontTweak” 修正 | CORRECTED → HYPOTHESIS | 源码未使用 FontTweak，修正效果未测 |
| iced 按钮内文字偏上 | CONFIRMED | iced-100/150：「保存」「复制」贴近按钮上沿；「可用 container 修正」为 HYPOTHESIS |
| Slint 居中正确 | CONFIRMED（限 100%/150% 静态截图） | 按钮、keycap、胶囊文字居中 |
| 1px 发丝线 / 选中框 / 8×8 手柄 @100% 三者精确 | CONFIRMED | 三图 y=51 处均为单像素 `#e6e5e0`；3× 放大图双线与手柄清晰 |
| 150% 三者“清晰” | **CORRECTED** | 取样：egui 发丝线变为 2 行半强度抗锯齿（`#eae9e5/#ebeae5`，发虚，外框同样 `#dddcd5+#cfcdc4`）；iced 吸附为 1 物理像素全强度；Slint 取整为 2 物理像素全强度（清晰但偏粗）。只有 iced/Slint 满足“清晰”，且线宽策略不同 |
| egui 150% “窗口不变，内容缩放” | CONFIRMED，但属测试方法差异 | `set_zoom_factor` 不改窗口尺寸，egui-150 画布被裁切、提示卡压住内容；不能据此判断 egui 的真实 DPI 布局 |
| Slint 无原生虚线，spike 拼接长度写错 | CONFIRMED | 26 段×5px 从 x=373 起覆盖到 ~503，遮挡块仅 374–476；slint-100 与对比图中虚线越过遮挡块伸到「手机」 |
| egui/iced 原生虚线 | CONFIRMED | `Shape::dashed_line`、`LineDash{3,2}` |
| Slint “最接近 DESIGN.md 写法” | CONFIRMED（写法层面），视觉层面 CORRECTED | `global T` 一一映射 token 属实；但 Slint 截图中工具组居中，与 HTML 样稿（品牌后左对齐、gap 4）不符，属 spike 布局实现偏差，非框架限制 |
| 焦点环、8px 圆角按钮、胶囊、keycap | CONFIRMED（静态） | 三者均可见橙色焦点环与圆角；但三者都未实现 §4.2 keycap 底边 2px，焦点环是硬编码而非真实键盘焦点 |
| 无障碍：iced 0.14 无 AccessKit | **CONFIRMED** | iced 0.14.0、iced_winit 0.14.1、iced_widget 0.14.2、iced_core 0.14.0 源码与 Cargo.toml 无 accesskit；spike 锁文件 accesskit 计数 0 |
| 无障碍：Slint winit 后端依赖 AccessKit | CONFIRMED | `i-slint-backend-winit` 可选依赖 accesskit 0.24 / accesskit_winit 0.33；`slint` 默认 feature 含 `accessibility`；锁内含 accesskit_windows |
| 无障碍：egui AccessKit | CONFIRMED（补充） | eframe 默认 feature 含 `accesskit`，但本 spike `default-features = false` 未启用，锁中无 accesskit_winit——spike 的 egui 二进制实际没有接入屏幕阅读器 |
| 许可证 egui MIT OR Apache-2.0 / iced MIT | CONFIRMED | egui、eframe、iced、iced_winit Cargo.toml |
| Slint GPL-3.0 或 Royalty-free 2.0 或商业 | CONFIRMED | `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`；免版税条款要求 AboutSlint 或公开下载页署名徽标；另禁止嵌入式、禁止对外暴露 Slint API、禁止单独分发 |
| 字重仅 Regular、Inter/JetBrains Mono 未测 | CONFIRMED | 三者只加载 NotoSansCJKsc-Regular + 系统 Consolas |

## 关键缺口

1. **125% 未测**（验收项明确要求 125%/150%）；150% 也只是框架缩放模拟，非系统 DPI / 每显示器 DPI 切换。分数倍率下线宽取整策略（见上）正是选型差异点，必须在真实 125%/150% 下复测。——高优先，可能影响结论。
2. **交互全未测**：指针拖拽选中框与 16×16 手柄命中、键盘焦点遍历、撤销重做刷新频率。——中，属框架共性能力，Slint/iced 预计可行（HYPOTHESIS）。
3. **大图纹理更新**：24MP 合成图每次编辑后回传 GUI 的耗时与内存（Slint `SharedPixelBuffer`→`Image`、femtovg GL 上传；iced `image::Handle`）。——**Slint 最可能的阻断点**，因 PRD U04 让画布完全依赖 Rust 合成图像。
4. **中文 IME**（文字标注工具）：三者均 NOT_VERIFIED。Slint `TextInput` / iced `text_input` 在 Windows 上候选窗位置与预编辑显示需实测。——中高。
5. **Slint 虚线**：无原生 dash，但遮挡对象虚线可由分段矩形或 Rust 覆盖层绘制，**不构成阻断**；需修正 spike 拼接算法并在 150% 下看段长取整。
6. **Narrator 实测**：DOCUMENTED ≠ 可用；手绘画布对象（遮挡块、标注）在任何框架下都需自建语义。
7. **渲染后端兜底**：Slint 默认 femtovg（OpenGL），需确认无 GL / 远程桌面下回退到 software 或 skia 渲染器的表现；egui/iced 走 wgpu，同样需兜底。
8. **测量口径**：内存差异混合了渲染后端（wgpu vs GL）与字体图集，不能直接归因于“框架更轻”；egui 的 411 MB 私有内存部分来自 CJK 字体图集 + wgpu，未拆分。
9. **egui 已有 POC 历史**（`poc/native-egui`）：迁移成本与已积累的证据未在 README 中权衡。

## 推荐与条件

我的推荐：**Slint 作为首选候选（有条件），iced 为备选，egui 降级**——与实现者的暂定排序一致，判断基本成立，但理由需修正与收窄：

- Slint 优势有截图/测量支撑：文字度量居中正确、分数倍率下线条不发虚、token 映射最直接、空闲内存最低、AccessKit 默认接入。
- iced 优势：MIT、原生虚线、150% 下 1 物理像素发丝线最贴设计稿；劣势：0.14 **无 AccessKit**（已核实），按钮内容默认不居中。对 Windows 桌面产品，无障碍缺失是实质性扣分，故作备选合理。
- egui 降级理由成立：CJK/Latin 混排基线错位、分数缩放下发丝线抗锯齿发虚、几乎全部控件需手绘（手绘控件无 AccessKit 语义）。但“降级”不等于排除，若 Slint 与 iced 都在大图/IME 上失败，egui 的 POC 历史仍是退路。

Slint 进入 Gate 03 选定前必须验证（每项给出 MEASURED 证据）：
1. 许可证决策书面化：接受 Royalty-free 2.0 的 AboutSlint 署名（或商业授权），并确认不会对外暴露 Slint API；ShareReady 暂定名不影响此判断。
2. 24MP `SharedPixelBuffer` 更新：单次编辑→重绘延迟与峰值内存，在真实 125%/150% 系统缩放下。
3. 选中框拖拽 + 8 手柄（可视 8×8 / 命中 16×16）交互流畅、无亚像素抖动。
4. 中文 IME 在 TextInput 中的预编辑与候选窗定位。
5. Narrator 能读出工具按钮、属性条字段、提示卡。
6. 修正虚线拼接与工具栏布局偏差后重截对比图；补 Inter + JetBrains Mono（需先获下载授权）与粗体。
7. 无 GL 环境的渲染器回退。
任一项（尤其 2、4）失败，则对 iced 做同样验证，并评估 iced 无障碍的补救路径。

## 结论

**ACCEPT_WITH_LIMITS**

- spike 证据真实、可复现（体积、依赖、行数、内存均已独立重测一致；截图与源码对得上），足以支持“Slint 首选、iced 备选、egui 降级”的**暂定**排序。
- 需更正：150% 下 egui 发丝线发虚（非“清晰”）、Slint 为 2px 取整；FontTweak/container 修正为 HYPOTHESIS；egui spike 未启用 AccessKit；依赖数口径含非 Windows 目标；Slint 工具栏布局偏离样稿。
- 不可作为最终框架选定：125% 未测、全部交互/IME/大图/Narrator 未测，且本审查为代理审查而非真人评审。

## 第二轮：Slint 条件验证审查

日期 2026-10-19　审查者：同一独立子代理（非真人）。对象：`VERIFY_SLINT.md`、`slint-verify/{src/main.rs,ui/verify.slint}`、`verify.ps1`、`ime_control.ps1`、`evidence/verify/*`。
方法：只从 `VERIFY update step=` 行重算统计（排除 `first_image_set_ms`）；核对 harness 坐标与 Rust 命中逻辑；读 i-slint-renderer-femtovg 1.18.1 源码确认通知时机；重跑 `ime_control.ps1` 一次（输出 `NOTEPAD text=[你好]`）。

| 声明 | 结论 | 依据 |
|---|---|---|
| 许可证：接受 Royalty-free 2.0，AboutSlint 已渲染 | CONFIRMED | 用户决定由协调方转述；截图与 UIA 中有 `#MadeWithSlint` / `Version 1.18.1`；产品 About 对话框尚未实现 |
| femtovg 100%：set 中位 56 / p95 62；到帧 107 / 114 / max 139 | CONFIRMED（小注） | 重算 n=20：set 中位 56.55（四舍五入应为 57）、p95 62.0；到帧中位 107.3、p95 113.6、max 139.2。n=20 时 p95 即第 19 个值，样本小 |
| femtovg 150%：104 / 116 / 144 | CONFIRMED | 中位 103.75、p95 115.9、max 143.6 |
| 软件渲染 set 中位 58，帧时间不可测 | CONFIRMED（需改述） | 中位 58.4。软件路径的 20 次 set 在**同一个**定时器回调里连跑，期间不渲染，所以只测到 CPU 合成 + 拷贝 + `set_image`，不含任何软件光栅化耗时 |
| 峰值内存 631–651 / 584 MB；软件 195 MB | CONFIRMED | 日志 630.5/583.7、650.7/584.2、195.0/195.3。取样间隔 100 ms，可能漏掉瞬时峰值 |
| 拖拽 3 组结果与 14px 未命中 | CONFIRMED | 起点 120,100,160,60：移动 +30/+20 得 150,120,160,60 ✓。右下手柄中心 (310,180)，按下点 (316,186) 偏 6/6：在 8px 可视（±4）之外、`abs<=8` 命中盒之内，结果 185×75 ✓。左上 +10/+10 得 160,130,175,65 ✓。(349,209) 距右下手柄 (335,195) 14/14 → None ✓ |
| “16×16 命中区” | CORRECTED（细节） | 代码为 `<=8`，实际是含边界的 17×17；只测了 6px 内、14px 外两个点，**未测 8/9px 边界**；该点同时也在选区外，所以没区分“手柄优先于主体” |
| 150% 拖拽 | CONFIRMED（限模拟缩放） | harness 按 `×Scale` 换算屏幕坐标，日志里的逻辑坐标一致（349.3/208.7 为取整误差） |
| 中文输入法 TextInput 提交“你好” | CONFIRMED（仅提交路径） | `keybd_event` 发的是虚拟键，只有经过输入法才会变成汉字；日志 `text=你好`，截图可见。**未测**：预编辑显示、候选窗位置、Esc 取消、候选选择 |
| 记事本对照有效 | 部分 CONFIRMED / UNSUPPORTED | 重跑得到“你好”，说明同样的按键确实走拼音转换，作为阳性对照有效。“未切换时为 `nihao `”的阴性对照在脚本中不存在，也没有日志 → UNSUPPORTED。另外 Shift 切换依赖输入法初始状态，测试较脆弱 |
| UIA：9 个 Button 中文名可聚焦、Edit“标注文字” | CONFIRMED | 3 份日志一致，共 23 个元素 |
| UIA 补充问题 | 新发现 | 每个按钮下还有同名 `Text` 子元素，Narrator 可能重复朗读；画布、选中框、遮挡对象没有语义；Tab 焦点遍历与 `accessible-action-default` 未测；Narrator 实际朗读未测（需真人） |
| 软件渲染：启动、拖拽、IME 正常；近邻缩小丢细线 | CONFIRMED | `software-100-interact.txt` 干净；截图中 200px 网格线大多丢失 |
| 污染运行说明 | CONFIRMED，另需补充 | `software-100-perf.txt` 在 `updates_done` 之后、harness 交互之前出现 (206,138)、(285,161) 两次非脚本按下，把选区拖成 572×405，之后 harness 的 4 次拖拽全部误判为主体。该文件无 `interact_begin` 标记。它引用的 `software-100-after.png` 已被后来的干净运行覆盖，与此日志不对应 |
| 计时方法“到下一帧渲染完成” | CORRECTED | femtovg 源码中 `AfterRendering` 在 `flush_to_output` + `submit_commands` 之后、`present_surface`（交换缓冲）之前触发。GL 是异步的，所以 to_frame 测的是“合成 + set + 等待下一帧 + CPU 侧提交 GL 命令（含驱动拷贝 96 MB 纹理数据）”，**不含 GPU 完成、交换缓冲和 vsync**，是可见延迟的**下限**。另外 `start` 在合成之前，所以上传 + 渲染的增量约 50 ms，而不是 107 ms |
| 24MP 约 0.1 s/次，不可逐帧重合成 | CONFIRMED（方向正确） | 结论保守合理；显示预览降采样方案为 HYPOTHESIS，尚未实测 |
| system-tray 默认特性 | HYPOTHESIS（已正确标注） | 未核实 |

### 剩余阻断项 / 必须补的证据
1. **真实系统 125%/150% DPI 与每显示器 DPI 切换**：仍 NOT_VERIFIED（需用户改显示设置），是本轮最大缺口。
2. 计时补测：在 `present` 之后或下一次 `BeforeRendering` 处打点，或者用 `glFinish` 版本测量；并实测“屏幕分辨率预览”方案的更新耗时（这是产品实际要走的路径）。
3. 输入法：预编辑、候选窗定位（在 125%/150% 下）、取消；补跑并记录阴性对照。
4. Narrator 真人朗读；去掉按钮的重复 Text 语义；Tab 遍历。
5. 命中边界 8/9px 与手柄优先级测试；无 GPU 真机。
以上均不是已证实的框架缺陷，目前**未发现 Slint 的阻断项**。

### 结论
**ACCEPT_WITH_LIMITS——Slint 可以选定为 Gate 03 框架**，但 Gate 03 记录中必须写明以下限制：
- 证据是代理自动化，在 100% 系统缩放 + `SLINT_SCALE_FACTOR` 模拟下取得；性能数字是 CPU 侧下限；Narrator、真实 DPI、IME 的预编辑与候选窗都未验证。
- 产品架构须采用“拖拽中由 Slint 画轮廓，松手后由 Rust 合成屏幕分辨率预览”的方式；24MP 全量上传只用于导出或局部查看。
- 上面第 1、3 项在 07 阶段前若失败，按原计划对 iced 做同样验证；iced 缺 AccessKit 的问题仍然存在。
