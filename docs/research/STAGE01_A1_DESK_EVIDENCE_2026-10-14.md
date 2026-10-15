# SR-RES-005｜A1 问题证据与反证核查（Desk evidence）

> **2026-10-14 · SUPPLEMENTARY RESEARCH · NOT USER INTERVIEWS · GATES A/B NOT RUN**  
> 本文目标不是证明 ShareReady 必须做，而是检验“普通人在截图发送前存在可重复、现有免费工具没有解决好的痛点”这一 A1 假设。收集到的帖子和官网不能代替亲自操作/招募访谈。  
> 本记录为 ChatGPT 阅读公开材料的定性摘录；不计算市场规模、讨论热度趋势或显著性。

## 1. 公开用户信号：证据价值必须区分

| 编号 | 日期与来源 | 谁在提出问题？ | 可直接支持的观察 | 不能推出的结论 |
|---|---|---|---|---|
| E01 | 2026-07-14，[r/software 寻找截图工具](https://www.reddit.com/r/software/comments/1uvxk82/looking_for_screenshot_apps/) | 用户自述需要快速截图、保存并发往 WhatsApp | 截图**分享链路**和剪贴板/保存摩擦是明确用户诉求 | 此贴未提出隐私遮挡，不能当截图脱敏高频证据 |
| E02 | 2026-08-11，[r/Compliance 客户资料截图](https://www.reddit.com/r/Compliance/comments/1vllp2y/how_do_you_deal_with_screenshots_containing/) | 提问者询问工单截图和客户信息的处理方式；回复为从业者情境 | 工单与截图资料分享有真实隐私风险，实践中有制度/人员判断因素 | 不代表普通社交用户普遍需要，也不证明独立 Web 工具是答案 |
| E03 | 2026-08-25，[r/chrome_extensions 工作流调查](https://www.reddit.com/r/chrome_extensions/comments/1vy755e/what_is_missing_from_your_current_browser/) | 发帖人为正在做扩展的开发者；其中一位回复者偏好先截屏后注释 | 至少有一名回复者重视保留原始画面，再导出扁平化的安全副本 | 问卷型推广帖有选择偏差，不可用点赞/回复当市场样本 |
| E04 | 2026-10-02，[r/privacy PDF/照片永久脱敏](https://www.reddit.com/r/privacy/comments/1wvmgef/how_do_i_safely_redact_sensitive_info_in_pdfs_and/) | 用户直接提问 | 永久去除数据（而非视觉叠层）对用户是可理解的问题 | PDF 处理与截图 PNG 不同，不能因热门讨论就把 PDF 纳入 MVP |
| E05 | 2026-10-03，[r/YouShouldKnow 黑框遮盖 PDF 风险](https://www.reddit.com/r/YouShouldKnow/comments/1wwd2p4/ysk_putting_a_black_box_over_something_in_a_pdf/) | 社区科普与大量评论 | 对“遮住不等于真正删除”的担忧有显著公众关注度，需明确产品导出语义 | 讨论对象主要为 PDF 文本层，不能据此声称我们的截图导出也会有同类可恢复漏洞 |
| E06 | 2026-08-09，[r/software Blackbar](https://www.reddit.com/r/software/comments/1vjz8xx/i_built_an_opensource_chrome_extension_that/) | 竞争产品开发者自述 | 还有竞品探索“截图之前检测并遮挡”的替代工作流 | 自我营销不证明真实用户需求或功能安全 |
| E07 | 2026-08-12，[r/SideProject PrivacyCam](https://www.reddit.com/r/SideProject/comments/1vmc8wz/i_built_an_ondevice_app_for_hiding_private/) | 竞争产品开发者自述 | 开源本地图片/视频/PDF 遮挡已有其他供给 | 自述功能未实际审计，不证明应用质量 |

**严格区分**：E01、E02、E04 为用户发起的公开问题；E03 的回复提供额外独立观点；E05 是相关安全教育；E06/E07 为**开发者推广**，主要应作竞品线索而非需求证据。所有证据仍是线上选择样本，不等于 10 位经同意的目标用户访谈。

## 2. 竞争威胁：不是只有两个 Web 遮挡工具

| 对照对象 | 有官方/公开页面支持的功能 | 对 ShareReady 形成的直接挑战 | 当前证据 |
|---|---|---|---|
| [Redacted](https://redacted.app/guides/how-to-redact-a-screenshot) | 复制/下载、遮挡与详尽分享前检查清单 | “新产品多一个检查列表”不足以形成护城河 | DOCUMENTED |
| [Redaction Tools](https://redactiontools.com/redact-screenshot) | 粘贴、自动检测、人工检查、复制/下载；还建议聊天截图用其 chat anonymizer | 聊天场景里的用户价值也已有专门工具切入 | DOCUMENTED |
| [Blackbar](https://www.reddit.com/r/software/comments/1vjz8xx/i_built_an_opensource_chrome_extension_that/) | 开发者宣称在浏览器页面预先识别/屏蔽再截图 | 如果用户不愿额外打开工具，前置捕获可能优于我们的事后编辑 | SELF-REPORTED / NOT VERIFIED |
| [PrivacyCam](https://www.reddit.com/r/SideProject/comments/1vmc8wz/i_built_an_ondevice_app_for_hiding_private/) | 开发者宣称移动端本地识别与隐私处理，免费档+可选付费 | 移动端分享前工具并非空白市场 | SELF-REPORTED / NOT VERIFIED |
| Windows/macOS/iOS 原生截屏编辑 | 操作系统通常直接嵌入用户现有分享流程；此处未逐版本核对具体功能 | **零切换成本**可能比免费新网页更有价值 | INFERENCE / REQUIRES OS VERSION TEST |

## 3. 对三个候选情境的研究排序（假设，非用户评分）

| 情境 | 切换到独立 Web 工具的理由需要证明什么 | 反证方式 | 建议 |
|---|---|---|---|
| 社交聊天截图 | 能处理多参与者、通知、头像等复杂区域，复核更少漏且不更费事 | 对照 Redaction Tools chat anonymizer 和原生截图工具 | **研究 P0**：保留大众定位，但不要默认优势 |
| 电商订单/售后咨询 | 能安全隐藏个人资料，同时准确保留商品、价格和问题信息 | 对照 Redacted 完整操作路径及用户习惯 | **研究 P0**：过遮挡也要计入失败 |
| 技术问题/客服工单 | 用户会分享含路径、密钥、账号名的调试截图 | 对照 ShareX / Blackbar；测试“截图之前编辑”对“截图之后编辑” | **研究 P1**：并非大众市场频率证据 |

**最容易推翻 A1 的证据：** 大多数真正需要处理的用户发现现有系统工具或成熟免费方案已经足够，且不愿额外打开网页；若此情况成立，应 `NO-GO` 或更改具体情境，而非通过 Rust 重写同质化功能。

## 4. 接下来的有效证据（尚未取得）

1. 区分“最近 30 天分享截图频率”和“其中实际需要遮挡的次数”；两者不能混用。
2. 同一名用户在原生截图流程与网页工具之间切换的真实摩擦，起点必须是**截图已在剪贴板**而非已经打开我们的页面。
3. 三组典型任务不只测是否有黑框，还要测误遮挡、遗漏、实际导出与重新分享行为。
4. 至少两种免费基线工具的**独立复现**，不能从官网文案推导性能。
5. 如果研究支持某个情境，才设计低保真 UX；没有必要提前绑定 Leptos 或导入大体量 GUI 框架。

## 5. 审查结论（严格限度）

- **需求存在性**：有外部公开独立讨论支持，但仅为定性线索。
- **需求频率**：`NOT_VERIFIED`。
- **竞品功能空白**：`NOT_VERIFIED`；已有功能覆盖十分广泛。
- **切换意愿**：`NOT_VERIFIED`。
- **是否授权真实 Stage 01 研究执行**：仍需固定版本的真实 `de.py` Gate A/B 预览及相应确认（详见 Issue #2）。
- **是否批准 Rust 开发**：**NO**。

**当前结果：`A1_DESK_EVIDENCE_EXPANDED / A1_HUMAN_STUDY_NOT_VERIFIED / GATES_BLOCKED`。**
