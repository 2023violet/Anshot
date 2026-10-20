# 07 最终独立审查 — ShareReady 0.1.0（Windows，Slint）

日期 2026-10-19　工作树：`main` @ `355c37c` + 未提交改动（main.rs 覆盖窗池/预热、app.slint 图标、.gitignore、scripts/、perf.ps1、release 证据）

## 身份
- 审查者：**独立 Claude 子代理（自动化，非真人）**，未参与实现。只读审查 + 运行单元测试 / 金样本 / 像素比对脚本；**未运行 GUI e2e / perf**（会驱动真实键鼠），未联网，未改动除本文件外的任何文件。
- 本审查不等于安全审计、真人可用性测试或发布批准。

## 1. 核实结果（07_UI_QA_REPORT.md）
本人复跑（2026-10-19，本机）：
- `cargo test --workspace --locked`：**85 passed / 0 failed**（core 10+7+9、editor 34+2、platform-windows 13、render 8、其余 2）。
- `CARGO_TARGET_DIR=spike/slint/target cargo test --locked --manifest-path apps/desktop/Cargo.toml`：**24 passed**（capture 4、display 1、canvas 3、controller 16）。
- `python tests/golden/verify_v1_baseline.py`：`SR_V2_00_FROZEN_GOLDEN_VERIFIED`（T01–T05 FULL_IMAGE_PASS）。
- 对 `tests/desktop/evidence/release/` 的剪贴板读回图重新跑比对脚本（源图 `spike/assets/shot.png`）：
  `clipboard.png` (521,143)-(599,163) 1560/265040 像素 0 差异 → PASS；`clipboard-capture.png` (518,143)-(596,163) 0 差异 → PASS；
  `clipboard-b3.png` 马赛克非调色板 0、箭头红 402、文字 23、区外 0 差异 → `B3_PIXELS_PASS`。
- `dist/.../ShareReady.exe` SHA256 = `38e4bb93…eaf892`，zip = `2c88c693…ad5bc5`，与报告一致；与 `spike/slint/target/release/shareready.exe` 相同。

| 报告声明 | 判定 | 说明 |
|---|---|---|
| 被测物 SHA256 | CONFIRMED（哈希）/ 关联为推断 | e2e.log、perf.log **不含 exe 身份**；仅凭时间戳（exe 21:16:33 → e2e 21:16:52–21:17:58 → perf 21:18:16）推断测的是该 exe |
| F01 快捷键/拖选/吸附/Esc，4 项 capture 单测 | CONFIRMED | e2e.log CAPTURE/SNAP/ESC 行；吸附只核对尺寸 620×430，未核像素。放大镜无任何证据（NOT_VERIFIED） |
| F02 命令行/拖放/粘贴、EXIF T05 | CONFIRMED | DROP/PASTE 行；T05 金样本通过 |
| F03/F04 标注与遮挡像素 | CONFIRMED | 见上述复跑；马赛克"与源无关"由 render 单测支撑 |
| F05 撤销重做 | CONFIRMED（弱） | e2e 撤销只有截图 05（目视无遮挡、"未遮挡"），日志无断言 |
| F06 裁剪 | CONFIRMED | 单测存在并通过 |
| F07 复制 PASS / 另存部分 | CONFIRMED | `save_new` 三项单测通过；真实另存对话框未测，报告已如实标注 |
| F08 Ctrl+滚轮 133% | CONFIRMED | 1.1³≈1.331，ZOOM 行一致 |
| F09 全键盘（1–6、Tab、Delete、Ctrl+Y/S/O 等） | CORRECTED | e2e 只覆盖 M/A/T、方向键、Ctrl+C/V/Z、Esc、数字输入；main.rs `key()` 路由无单测；Ctrl+S/O 从未执行 |
| A2 遮挡区与原图无关 | CONFIRMED | 实心逐像素 + 马赛克调色板 + 单测 |
| A3 "3 个场景 0 差异" | CORRECTED | 前 2 个为全图逐像素；B3 场景标注区只计红像素数，非逐像素等于画布；alpha 未查（报告已注明） |
| A4 导出不含元数据 MEASURED | CORRECTED → DOCUMENTED/单测 | 依据是代码（`decode_generated_png` 头/块检查）与单测；未对 release exe 实际导出的 PNG 做块检查（另存未 e2e，剪贴板经 WinForms 重编码） |
| A6 全键盘 PASS（机器） | CORRECTED → PARTIAL | 02 §5.2 的 P0 范围含"调整"，但键盘无法调整尺寸（见缺陷 D2） |
| 性能数字 805/817/782、55/35/37、117、369、420MB | CONFIRMED（与 release/perf.log 一致） | 方法偏差：计时在 `Key` 返回后才开始（已含 30 ms 按键保持），快捷键与复制约低估 ≥30 ms，结论（<200 ms、<1 s）不变；第 2、3 次启动非冷启动；另有更早一次 `evidence/perf/perf.log`（75/37/36、249 ms）未在报告中说明 |
| "未预热时约 1.5 s" | UNSUPPORTED | 无对应日志 |
| 运行期网络端点 0 | CONFIRMED（采样） | 仅在 5 个时点采样 TCP/UDP，非全程抓包 |
| "reqwest/hyper/tokio/rustls 均不在锁文件" | CORRECTED | `apps/desktop/Cargo.lock` **含 tokio 1.53.3 与 zbus**（Linux AT-SPI 链）；`cargo tree --target x86_64-pc-windows-msvc -i tokio` 为空，即 Windows 构建不编译它们 |
| exe 约 49 MB | CONFIRMED | 48,912,896 字节 |
| 证据隐私（只截合成区域） | CONFIRMED（抽查） | 抽查 03/04/05/08/11 均为应用或合成图内部；"已删除 4 张"无法核实 |

## 2. 安全不变量
| 不变量 | 结论 | 依据 |
|---|---|---|
| 遮挡像素不含源信息 | 成立 | `render_into` 先整像素替换（实心色或 `mosaic_color(x,y)` 仅取位置哈希），再画标注；实心强制不透明；预览与导出同一渲染器 |
| 导出 == 当前版本 | 成立 | `prepare_render` 绑定 stamp+source 指针；`review_finished` 先校验 busy key，再 `review_is_current`→`accept_review`（含尺寸）→`finish`→`begin`→`bytes_for`，均在 UI 线程同步完成；任何编辑 `invalidate()` 使旧结果作废；跨文档由 document_id+`Arc::ptr_eq` 隔离 |
| 冻结整屏像素不落盘 | 成立（内存内） | 只存在于 `Session.frozen` 与覆盖窗图像；`end_capture` 清空图像并丢弃 Session。无写盘路径。注意 D1 发生时会话及冻结像素会滞留内存直至退出；常驻实例复制后文档仍留在内存（未写盘） |
| 保存永不覆盖 | 成立 | `write_file`→`save_new`，单测 `existing_file_is_never_changed` / ADS / 部分写入失败 |
| 无网络 | 成立（Windows 目标） | 源码无网络调用；Windows 依赖树无 tokio/hyper 等；运行期采样 0 |
| V1 冻结路径未改 | 成立 | `crates/shareready-core` 最后改动 ee28734、`e1-spike` 最后改动 d20c5ff，均早于本次重设计；金样本复跑通过；B3 仅新增 `tests/golden/render/scene-v1.png`，未替换冻结金样本 |

## 3. 缺陷（按严重度；均为代码推理，未在 GUI 复现）
- **D1 中 — 预热定时器可隐藏正在使用的覆盖窗，截图功能卡死至重启。** `main.rs:643` 在 800 ms 预热，`main.rs:441-443` 显示池窗并在 300 ms 后无条件 `hide()`。若用户在启动后约 0.8–1.1 s 内按 Ctrl+Shift+A，`begin_capture` 复用同一池窗（`main.rs:407-414`），随后定时器把它隐藏；`session` 仍为 Some，`main.rs:383` 使之后快捷键与托盘截图全部被忽略，Esc 也到不了隐藏窗口。修复：定时器回调中仅当 `session.is_none()` 时隐藏，或在 begin_capture 中取消预热。
- **D2 中低 — 键盘无法调整对象尺寸，与 02 §5.2 P0 键盘范围不符。** 方向键只调用 `nudge`（移动，`main.rs:319-322`）；属性条 X/Y/W/H 为只读 `NumberField`（`widgets.slint:103-107`，`app.slint:147-150`），外观像输入框却不可编辑。A6 应为 PARTIAL。
- **D3 低 — 覆盖窗池索引在窗口创建失败时可 panic / 错位。** `main.rs:398-407`：若某显示器 i 的 `CaptureOverlay::new()` 失败则 `continue`，下一显示器 i+1 新窗被 push 到下标 i，随后 `self.overlay_pool[i+1]` 越界 panic；同时 `overlay_pointer` 用池下标 i 查 `self.overlays.get(i)`（`main.rs:448`），两者在有 `continue` 时会错位。触发需 GL/窗口创建失败，概率低。
- **D4 低 — "复制后退出"标志可跨文档残留。** `dialog_choice` 在 copy-close 开始后设 `after_copy=Quit`（`main.rs:517`）；若渲染完成前用户确认替换图片，`accept_source` 清空 busy（`controller.rs:144`），结果走 `Notice::Ignored`（`main.rs:255`）不复位 `after_copy`；之后任意一次成功复制会直接 `quit_event_loop`（`main.rs:266`）。窗口很窄（数百毫秒）。
- **D5 低 — 打开图片过程中确认截图会被静默丢弃。** `confirm_capture` 已结束会话后调用 `spawn_open`，而 `spawn_open` 在 `ctl.opening` 为真时直接返回（`main.rs:526`），无提示。
- **D6 低（UX，可能）— 另存为选择已存在文件时，系统对话框会先询问"是否替换"，确认后应用再拒绝写入**（`platform.rs:13-15` 未关闭覆盖提示；`write_file` 拒绝）。安全但令人困惑；依赖 rfd 默认选项，未实测。
- 其他观察（非缺陷）：`with()` 借用失败时静默丢弃事件，当前代码未发现在借用内运行嵌套消息循环的路径，但 worker 回调若被丢弃会使 busy 永久卡住，属残余风险；`screen.rs` 的 FFI（句柄 Drop 守卫顺序、位图选出后再 GetDIBits、回调指针生命周期、热键线程消息循环）未发现问题。

## 4. 未验证项
- 真人：A1 计时、A7 主观评审、全部真人操作（**NOT_VERIFIED**）。
- 多显示器、负坐标真实布局、125%/150% 与混合 DPI（仅单测 + 单屏 100%）；Windows 11；无 GPU。
- 真实另存为对话框与 `save_new` 端到端；导出文件实际 PNG 块检查；剪贴板 alpha。
- 托盘菜单真实点击、托盘退出确认流程、快捷键冲突提示、放大镜、Explorer 真实拖放、中文输入法组字、Narrator 朗读。
- 本审查未运行 e2e/perf；上述 GUI 结论依赖实现方日志与截图（未含 exe 身份）。
- 独立安全审计、模糊测试、恶意图片样本、代码签名、许可证法律审阅（THIRD_PARTY_LICENSES 为由锁文件生成的 335 项清单，未附各许可证全文）。

## 5. 如实标注检查
- 07 报告、06 实现文档均明确"代理执行，非真人测试"，A1/A5/A7、Narrator 标 NOT_VERIFIED，Gate 07 写明"代理确认"——**符合**不虚报真人测试的规则。
- 需更正：上文 CORRECTED / UNSUPPORTED 各项（tokio、A4 证据等级、A6、F09、1.5 s）。
- 文档同步缺口：`README.md` 仍称"Windows 桌面使用 WinForms + WebView2"，产品目录未列 `apps/desktop`、`crates/shareready-render`，未提 0.1.0 包；关闭阶段需同步（closing-out）。
- 06 所述"用户 /goal 授权代理确认"本审查无法核实，仅为 DOCUMENTED。

## 结论：**ACCEPT_WITH_LIMITS**
作为"本地、未签名、未发布、代理机器验证"的 0.1.0 构建可接受：核心安全不变量在代码与测试层面成立，单测/金样本/像素比对可复现。限制：D1 应在任何分发前修复（低成本）；D2 需修复或把 A6 降为 PARTIAL；07 报告按第 1 节更正；真人、多屏/DPI、另存端到端仍为 NOT_VERIFIED，不得表述为发布就绪。


---

## 实现方处理记录（实现代理撰写，非审查者；2026-10-19）
| 缺陷 | 处理 | 验证 |
|---|---|---|
| D1 预热计时器隐藏使用中的覆盖窗 | 计时器只在没有截图会话时隐藏 | 代码审阅；e2e 截图/吸附/Esc 复测通过 |
| D2 无键盘调整尺寸 | Ctrl+方向键调整遮挡与框/高亮的右下边、箭头头部（Shift ×10） | 单测 `keyboard_resize_moves_the_far_edge_of_masks_and_frames`；e2e Ctrl+→×5 后区域 (518,143)-(601,163) 逐像素一致 |
| D3 覆盖窗创建失败索引错位 | 任一显示器失败即整体中止截图并提示 | 代码审阅（无法在本机制造失败） |
| D4 复制后关闭标志跨文档残留 | 换图时重置 | 代码审阅 |
| D5 打开中确认截图被静默丢弃 | 显示“正在打开上一张图片，请稍后再试” | 代码审阅 |
| D6 另存对话框先问替换再拒绝 | 同名时自动改为“名称 (2).png”等并提示实际文件名，仍不覆盖 | 单测 `free_path_never_returns_an_existing_file` |
| 报告更正 | 07 报告更新：tokio/zbus 仅 Linux 路径、A4 降为“文档+单测”、A6/F09 改为部分、A3 标注区有限核验、计时偏差说明、预热前数据来源、exe 身份写入日志 | `07_UI_QA_REPORT.md` v1.1 |

复测（新 exe `e6f9c5f5…768e`）：根 85 passed、desktop 26 passed、冻结金样本通过、e2e 全部 PASS（含新增 KEYBOARD）、perf 网络端点 0。
