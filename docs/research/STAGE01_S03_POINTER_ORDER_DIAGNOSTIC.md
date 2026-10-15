# SR-RES-007｜S03 连续手动遮挡的操作顺序与事件诊断

> **2026-10-15 · THIRD-PARTY LIVE BROWSER EVIDENCE / SAME-AGENT ANALYSIS / S03 NOT ACCEPTED**  
> 本文补充 [S03 首轮报告](STAGE01_S03_LIVE_BROWSER_PILOT_REPORT.md)。全部输入来自完全虚构的 T01/T02 截图。结果来自 Chromium 自动化与实际第三方导出 PNG，不是普通用户操作，也不能据此认定竞争产品存在漏洞。

## 1. 核心结论

**T01 已证明存在可完成的操作顺序；T02 在已测试的顺序、手势节奏与虚拟桌面条件下仍无法由当前 Playwright 脚本连续新增第二个手动遮挡。**

这意味着不能继续将前一轮 Redaction Tools 的自动化漏遮挡记录视为“产品功能缺失”。完整且公平的 S03 竞品基线仍处于 `IN_PROGRESS`；与真人使用行为、产品缺陷及 A1 竞争优势相关的推断均为 `NOT_VERIFIED`。

## 2. 逐条可复核实验

| 试验 | 环境 | 操作顺序 | 机器直接观察 | 归因限度 |
|---|---|---|---|---|
| [#38686065063](https://github.com/2023violet/ShareReady/actions/runs/38686065063) T01-C0 | Headless Chrome；记录版本见事件日志 | 头像→姓名→通知 | 页面 `2 hidden`；导出仅头像和自动检测电话改变 | 姓名和通知的后续脚本手势未成为有效框 |
| 同一 Run T01-C1 | 同一 Runner 版本 | **姓名→头像→通知** | 页面 `4 hidden`；真实导出中 4/4 个敏感区域中心改变，两个保留区像素不变 | 证明工具在本次环境中能够完成 T01，不证明一般用户完成率 |
| 同一 Run T01-C2 | 同一 Runner 版本 | 通知→姓名→头像 | 页面 `3 hidden`；导出姓名未改变 | 与脚本手势顺序相关，不能直接归咎于 OCR |
| [#38686424601](https://github.com/2023violet/ShareReady/actions/runs/38686424601) T02 | Headless Chrome **155.0.8059.39** | 地址→收件人 | 真实 PNG 中地址、自动识别电话改变，收件人中心未改变；必须保留的订单内容未改变 | 同一问题反向出现；该次 `hidden_count` 正则未匹配，应只依赖文件像素证据 |
| [#38686589480](https://github.com/2023violet/ShareReady/actions/runs/38686589480) T02 | Headless Chrome **155.0.8059.39** | 地址→收件人、收件人→地址；各做正常速度/额外停顿 | 四个全新会话均为 `2 hidden`；每次仅第一笔手动框增加计数 | 延长停顿没有解除，不证明真人也失败 |
| [#38686749895](https://github.com/2023violet/ShareReady/actions/runs/38686749895) T02 | **Xvfb 可见窗口 Chrome 154.0.8037.97** | 地址→收件人、收件人→地址 | 两个会话仍为 `2 hidden`；图像和页面事件已存档 | 非 Headless 也复现，但浏览器版本与前组不同，不能把模式差异当纯粹 Headless 影响 |

T01 三种顺序的实际导出像素审查：[Run #38686355568](https://github.com/2023violet/ShareReady/actions/runs/38686355568)，三个导出文件对比结果：

- C0：2/4 目标中心改变，保留区 2/2 未改变。
- **C1：4/4 目标中心改变，保留区 2/2 未改变；导出 PNG 未见 `exif` 信息。**
- C2：3/4 目标中心改变，保留区 2/2 未改变。

验证的是预先指定的中心点和需保留区域，不是逐像素全面安全审核。自动化脚本知道答案，任何这类耗时均不能代表真人研究对象的耗时。

## 3. 输入事件诊断：已知与未知

在 [#38686065063](https://github.com/2023violet/ShareReady/actions/runs/38686065063) 的某些失败序列中，文档捕获监听器观察到 `pointerdown` 和 `mousedown`，却未观察到配对的 `pointerup`。目标元素的初始 hit-test 为 `div.absolute.inset-0.cursor-crosshair.touch-none`；成功生成的红action框覆盖层表现为 `button.absolute.rounded-[2px]`。这个日志支持“手势生命周期异常”的**假设**，不能定位到浏览器、React pointer capture、重绘还是脚本的具体责任。

浏览器测试可重复性还受约束：Runner 的 Chrome 随 GitHub 镜像更新，从 154 升至 155。未来公平测试要固定 Chromium 构建版本或记录严格的每次真实运行版本，禁止把不同版本的耗时混合做统计检验。

## 4. 二进制输出与可信证据

- [三顺序真实编辑/导出及 DOM 事件 Artifact #11950006511](https://github.com/2023violet/ShareReady/actions/runs/38686065063/artifacts/11950006511)（GitHub ZIP SHA256：`929703fafb999f65c27a807ed814be68c251c5f06a6ef00bdb6d5e0f13a702bb`）。
- [三顺序导出文件比较拼版 Artifact #11950366319](https://github.com/2023violet/ShareReady/actions/runs/38686355568/artifacts/11950366319)。
- [T02 地址优先真实 PNG Artifact #11949882174](https://github.com/2023violet/ShareReady/actions/runs/38686424601/artifacts/11949882174)。
- [T02 延时/不同顺序测试 Artifact #11950686585](https://github.com/2023violet/ShareReady/actions/runs/38686589480/artifacts/11950686585)。
- [T02 Xvfb 可见窗口 Artifact #11950876578](https://github.com/2023violet/ShareReady/actions/runs/38686749895/artifacts/11950876578)。

Artifacts 的平台保留期约 30 天，证据下载要检查实际摘要，不能使用过期 URL 冒充文件仍存在。

## 5. 必须执行的独立人工交互复核

下一步需要**真人操作软件**（不是自动化 Headless 或 Xvfb）：

1. 在普通 Windows / macOS 桌面 Chrome/Edge 中打开 <https://redactiontools.com/redact-screenshot>；记录 OS、浏览器精确版本和测试时间。
2. 从仓库上传 [T02_ORDER.png](fixtures/T02_ORDER.png)，保持 `AI OFF` 与 `Solid box (safest)`，观察默认隐藏条数。
3. 依次框选收件人、地址；保存第一次及第二次后隐藏计数，实际下载 PNG；新会话调换框选顺序再做一次。
4. 检查真正 PNG 是否覆盖收件人、演示电话和地址，同时仍保留商品/金额/问题。保存**只有虚构数据**的演示录屏、截图和导出文件，记录失败步骤。
5. 如果真人两种顺序均正常，归为 `AUTOMATION_HARNESS_INCOMPATIBLE` 并修正 Playwright 测法；如果真人稳定复现，再有资格复核第三方工具的真实交互缺陷，但不得发布缺乏独立审计的产品漏洞判断。
6. 与 Redacted 基线复测在固定的浏览器版本和相同输入条件下执行；**不以脚本已知真值的时间**替代真实用户效率指标。

**当前 S03 不能 DONE：`T01_REDACTION_TOOLS_COMPLETE_IN_ONE_SCRIPTED_ORDER / T02_MULTIMASK_AUTOMATION_UNRESOLVED / NO_REAL_HUMAN_RETEST / A1_OPEN`。**

## 6. 权限与范围

本阶段用户只批准研究流程；未授权 Rust 产品实现、发布、合并 PR。Skill 的真实状态继续是 `Gate A/B PASS; S01/S02 DONE; S03 IN_PROGRESS`。不得伪造 `de.py step 3 --status done`、参与者、人工复核结果或新的 GO 决策。
