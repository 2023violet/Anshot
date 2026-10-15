# SR-S01｜A1 真实问题证据启动记录

**日期：2026-10-14 (PDT) · 状态：S01 IN PROGRESS · 外部信号复核，不是用户访谈，也不是产品竞品实测**

## 任务合同

Gate B 计划步骤 1 需要：“至少两个有日期、独立可归属的真实用户问题陈述；区分开发者推广；说明真实招募是否可行”，并核查是否有足以改变现有免费方案使用习惯的价值缺口。原研究仅有公开网页，这里进一步核对来源，**不使用 AI 编造参与者**。

## 1. 本轮访问并核对的网页记录

| # | 来源 / 事件日期 | 首发主体与原话性质 | 能支持的事实 | 证据边界 |
|---|---|---|---|---|
| U-01 | 2026-08-11 [r/Compliance: How do you deal with screenshots containing customer data?](https://www.reddit.com/r/Compliance/comments/1vllp2y/how_do_you_deal_with_screenshots_containing/) | 社区提问者讨论支持工单、Bug 报告和内部文档中的客户信息，询问如何正式脱敏；回复者提出实际截图中的可见数据与图层风险 | **用户自发提出**的截图敏感资料管理问题，偏工作/合规 | 不证明普通人高频打码，更不证明需要额外打开独立 Web App；用户分享频率未知 |
| U-02 | 2026-09-14 [r/postcrossing: highlighter may reveal personal information](https://www.reddit.com/r/postcrossing/comments/1wgcjuo/psa_using_highlighter_feature_can_still_show/) | 社区参与者提醒明信片背面地址打码使用高亮笔可能被再次调整曝光，表达真实的隐私担忧 | **独立个人分享场景**中遮挡手段安全性和二次处理风险确实有人关注 | 属于照片/明信片，而非典型截图；是否愿意使用专业工具未知 |
| U-03 | 2026-07-14 [r/software: Looking for screenshot apps](https://www.reddit.com/r/software/comments/1uvxk82/looking_for_screenshot_apps/) | 提问者抱怨 Windows 系统截图保存和 WhatsApp 分享不便；后续回复提问截图软件是否可以自动屏蔽不想包括的内容 | 既涉及**截图分享链路摩擦**，也有具体自动遮挡询问 | 主帖的直接需求是保存/分享方便，不能算具体遮挡工作流频率证明 |
| U-04 | 2026-10-02 [r/privacy: safely redact PDFs and photos](https://www.reddit.com/r/privacy/comments/1wvmgef/how_do_i_safely_redact_sensitive_info_in_pdfs_and/) | 提问者需要真正永久的数字文件脱敏 | 对像素替换/导出真实性的用户教育有参考 | PDF/照片非 ShareReady 当前 MVP 范围，不得扩张功能 |

## 2. 明确不能计入的“真实需求样本”

- [r/software: Blackbar extension promotion](https://www.reddit.com/r/software/comments/1vjz8xx/i_built_an_opensource_chrome_extension_that/) 为**工具作者推广及征求反馈**；只能记录替代工作流/竞争风险，不算独立用户访谈。
- [r/SideProject: PrivacyCam](https://www.reddit.com/r/SideProject/comments/1vmc8wz/i_built_an_ondevice_app_for_hiding_private/) 同理，是开发者自述，不算实测和真实需求频率。
- [r/iScanner: screenshot privacy guide](https://www.reddit.com/r/iScanner/comments/1vj66nd/how_to_blur_sensitive_information_in_screenshots/) 带有厂商推广意图，不作为独立行为证据。

## 3. 对 A1 的暂时判断（不等于 GO）

- **问题确实有人公开提及：** U-01、U-02 均为可追溯的非产品推广表达，但覆盖情境相差较大。
- **近 30 天真实出现的遮挡任务频率：NOT_VERIFIED**（没有合格访谈，N=0）。
- **从 ShareX、Redacted、系统截图工具切换的实际意愿：NOT_VERIFIED**。
- **竞品上相同任务的漏遮挡、完成时间与导出可信度：NOT_MEASURED**。
- **独立用户招募可行性：NOT_VERIFIED**。公开社区存在潜在线索，但当前未联系任何人、未取得参与同意。若未能取得真实招募途径，则下一步只能记录 BLOCKED_BY_RECRUITMENT；不得臆称“可招到十人”。
- **当前愿意参与本产品研究的人数：0（未招募）；已完成同意会话：0**。
- **假设更新：A1 OPEN；没有通过条件；不存在凭网上帖子直接启动 Rust 工程的依据。**

## 4. 下一步证据要求

1. 重新查阅来源时间、作者与发帖场景；如果删除/过期则记录链接失效，不能替换为不存在的引文。
2. 完成隐私保护的招募渠道可行性评估：参与者可在不用提交私人截图的情况下使用**合成** T01/T02。
3. 在明确被试同意后，由其他评审者核实是否真正存在“重复发生 + 现有工具缺口”的证据。
4. 研究者需单独记录抽样偏差：公开发言者自选择、行业群体偏差、系统截图工具用户已有的零切换成本。
5. **未取得实际可行的研究执行途径和至少两条被核查的独立需求信号之前，S01 不得标记 DONE。**

**证据性质：`DESK_SOURCE_VERIFIED / RECRUITMENT_NOT_VERIFIED / USER_SESSIONS_N=0 / COMPETITOR_HANDS_ON_N=0`。**
