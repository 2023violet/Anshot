# SR-RES-008｜S03 两款真实竞品同任务复测：完整导出基线

> **2026-10-15 · REAL THIRD-PARTY BROWSER AUTOMATION / RESEARCH ONLY · SAME-AGENT REVIEW**  
> **任务级结论：S03 证据已达到候选验收条件，尚待原版 Skill 运行正式状态迁移。** 本文件不能作为 ShareReady 产品安全、真人完成率或竞品优劣排名的结论。

## 0. 可纠正先前误判的重要事实

此前 Redaction Tools 在连续脚本拖拽后，T01/T02 有敏感区域未被遮挡。现在确定**这组自动化失败并不代表工具无法完成这些任务**。使用“第一处手动遮挡 → 点击编辑画布外空白位置 → 第二处遮挡”的真实浏览器输入方式，四份导出均实现了全部预设目标中心像素被替换、指定保留区域未变化。

- T01 中“姓名→头像→通知”顺序曾独立成功；详见 [SR-RES-007](STAGE01_S03_POINTER_ORDER_DIAGNOSTIC.md)。
- T02 的“第二处框选失败”在 [八组新的受控实验](https://github.com/2023violet/ShareReady/actions/runs/38688284674) 中得到解决：两种顺序均可通过中间的空白点击完成。按 Esc、移出画布、增高浏览器视口均未解除该次自动化问题。
- 事件采样中，失败的第二笔表现为 `pointercancel`，而使用空白点击后第二笔观察到 `pointerup`。该机制是**脚本/浏览器/应用共同作用的诊断**，不能归因成竞品安全漏洞。
- 因此，本报告**取代**此前将旧 2/4、2/3 自动化文件当做竞品本体漏检率的任何解读。旧证据保留，但用途仅为失败实验和交互兼容性案例。

## 1. 实际运行环境、样本与原始二进制

| 参数 | 确定证据 |
|---|---|
| 工具 A | <https://redacted.app/>，网页内部构建版本 `UNKNOWN_WEB_BUILD` |
| 工具 B | <https://redactiontools.com/redact-screenshot>，网页内部构建版本 `UNKNOWN_WEB_BUILD` |
| 设备 | GitHub Actions 托管 Linux；**Chrome 154.0.8037.97**；Playwright Python 1.55.0；headless；1280×900；en-US |
| 可选 AI | 两款均未启用额外 AI 功能；Redaction Tools 默认规则识别电话保留启用 |
| 任务源 | [manifest.json](fixtures/manifest.json) + T01/T02 原始 PNG 哈希；全部虚构 |
| 操作 | 预知真值坐标（oracle-assisted），不是用户自行发现隐私区域 |
| 手动框选 | Redacted T01:4/T02:3；Redaction Tools T01:3+自动电话/T02:2+自动电话 |
| 特殊输入 | Redaction Tools T01 在连续手动框之间额外点击空白处 2 次；T02 1 次 |
| 首次故障隔离 | [run 38688284674](https://github.com/2023violet/ShareReady/actions/runs/38688284674)，[八组证据 Artifact #11950219402](https://github.com/2023violet/ShareReady/actions/runs/38688284674/artifacts/11950219402)，ZIP SHA256 `6d34b00640d3d3cbb7840cb4716f38181e4a67442c8f534b291035f7c0d7b793` |
| 新的完整两工具实测 | [run 38688552378](https://github.com/2023violet/ShareReady/actions/runs/38688552378) |
| 实际真实第三方导出文件 | [Artifact #11951019664](https://github.com/2023violet/ShareReady/actions/runs/38688552378/artifacts/11951019664)，ZIP SHA256 `ce05e3497aff6a2dae2ad3472610b7cdd42e2441481cbf66cedcc50fe5bd2d45` |
| 第二 CI 独立重新读出并绘制对照图 | [run 38688790908](https://github.com/2023violet/ShareReady/actions/runs/38688790908)，[Artifact #11951625434](https://github.com/2023violet/ShareReady/actions/runs/38688790908/artifacts/11951625434)，ZIP SHA256 `a35484e899423f47efa161a46e845708e716f4862e4d3efaec7476d83c92ab4a` |

导出文件可在上述 Artifact 中获取。所有 Artifact 预计在 2026-11-14 UTC 到期，过期后须重新运行并记录新哈希。

## 2. 四份真实 PNG 文件与像素检查

| 工具 | T01：4 处真值目标 | T02：3 处真值目标 | 规定保留区域 | 导出 |
|---|---|---|---|---|
| Redacted | 4/4 中心变为 RGB(0,0,0) | 3/3 中心变为 RGB(0,0,0) | T01 2/2；T02 1/1 保留区域像素一致 | PNG，真实浏览器下载成功 |
| Redaction Tools | 4/4 中心变为 RGB(17,17,17) | 3/3 中心变为 RGB(17,17,17) | T01 2/2；T02 1/1 保留区域像素一致 | PNG，真实浏览器下载成功 |

上述数值经两个**分离的 CI 进程**检查：第一轮脚本保存真实文件并计算目标/保留像素差异，第二轮从第一轮 Artifacts 读取相同文件，再次核验四组任务的目标区域与保留区，并做图像拼版。第二轮同时检查导出文件未携带 EXIF 信息。本会话已实际查看拼版，指定目标的显示文字已被遮盖，保留内容仍可见。

**安全结论限制：** 已核验真值框内部的像素变化、中心颜色以及必须保留区域的差分；不是全部敏感文字字符级恢复测试，也未对每个真值框做全面安全形式证明。没有验证浏览器剪贴板重贴、导出尺寸极限、移动端浏览器、现代浏览器完整隐私威胁模型。**不能将研究夹具 PASS 换算为“绝对安全”。**

## 3. 自动化时间只做内部诊断

| 真实操作 | 自动化诊断秒数（含固定等待） | 额外中间空白点击 |
|---|---:|---:|
| Redacted T01 | 8.895 | 0 |
| Redacted T02 | 8.046 | 0 |
| Redaction Tools T01 | 8.457 | 2 |
| Redaction Tools T02 | 7.302 | 1 |

这些数据是**单次 Playwright 执行**的时间，脚本提前知道敏感信息在哪里，并且两款工具使用不同自动/手动组合以及固定等待。不是用户任务完成时间，不构成对效率、低遗漏率或竞争优势的统计结论，也不应计算“谁快 X%”。

原始机器可读记录： [STAGE01_S03_MATCHED_EXPORTS.csv](STAGE01_S03_MATCHED_EXPORTS.csv)。

## 4. 网络观察边界

在该测试脚本监听的会话请求中，Redacted 未记录到 POST；Redaction Tools 记录到至 `api.openpanel.dev` 的 POST。没有截取 payload，不能推导谁上传了什么，也不能直接评价工具隐私保障。对外安全宣传前须再做独立网络审计。

## 5. S03 可验收与未解决事项

| S03 原计划验收项 | 本轮结论 |
|---|---|
| 同一测试协议，≥2 个实际竞品 | **PASS**：Redacted、Redaction Tools |
| T01/T02 同任务全流程，实际 PNG 导出 | **PASS**：4/4 真正完成，并保留二进制 |
| 可追溯运行时间、浏览器/系统版本和事件记录 | **PASS**，但网站内部构建版本未披露，已记 `UNKNOWN_WEB_BUILD` |
| 真值区域与保留区域的独立机器过程检查 | **PASS（任务级、非全面安全审计）** |
| 自动化脚本自身错误排除 | **PASS（本两任务通过空白点击实现；其他工具/版本不做外推）** |
| 真实用户时间、主观信任、是否发现全部敏感信息 | **NOT_MEASURED**；属后续招募/会话阶段 |
| 是否存在产品差异化优势 | **NOT_VERIFIED；A1 仍为 OPEN** |
| 研究外的 Rust 产品实现/PR 合并/发布 | **NOT AUTHORIZED** |

**建议：经固定 Skill 的真实结构和证据检查后，只将计划中的 S03 任务标记 DONE；随后进入 S04 研究伦理与招募准备。** 不得因为 S03 自动化任务完成就宣告 A1 需求验证成功，S05 的真实受试者人数依然为 0。
