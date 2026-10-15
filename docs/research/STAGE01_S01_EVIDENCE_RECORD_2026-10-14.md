# SR-S01｜需求信号与招募可行性复核（验收候选）

**评审时间：2026-10-14 PDT / 2026-10-15 UTC**  
**类型：公开来源的桌面研究（不是访谈、不是竞品实测）**  
**关联门禁：Gate A=PASS、Gate B=PASS；机器步骤 1 原状态=in_progress。**

## 1. 核验范围与证据分类

| 编号 | 日期 / 原帖 | 发言主体 | 对 A1 的实际支持 | 限制 |
|---|---|---|---|---|
| P01 | 2026-08-11 [r/Compliance：截图中的客户信息如何处理](https://www.reddit.com/r/Compliance/comments/1vllp2y/how_do_you_deal_with_screenshots_containing/) | 公开提问者，非本研究受试者 | 直接询问工单/bug 报告/内部材料中客户数据截图如何脱敏；相关需求确实有人讨论 | 偏企业合规；看不到使用频率和迁移独立 Web 工具意愿 |
| P02 | 2026-07-14 [r/software：寻找截图软件](https://www.reddit.com/r/software/comments/1uvxk82/looking_for_screenshot_apps/) | 公开提问者及其随后独立回复 | 原帖是截图保存和 WhatsApp 分享摩擦；同一提问者在开发者介绍截图+编辑工具后追问能否“自动屏蔽不想包括的东西” | 遮挡仅为跟帖提问，不证明其亲身遭遇泄露，也不证明希望手动框选 |
| P03 | 2026-09-14 [r/postcrossing：高亮遮挡风险](https://www.reddit.com/r/postcrossing/comments/1wgcjuo/psa_using_highlighter_feature_can_still_show/) | 独立分享者，非供应商广告 | 公众担忧透明高亮无法可靠遮挡私人地址、存在漏遮挡风险 | 场景是明信片照片而不是截图；只作安全邻近证据，不充当截图用户 |
| P04 | 2026-08-09 [Blackbar 开发者推广](https://www.reddit.com/r/software/comments/1vjz8xx/i_built_an_opensource_chrome_extension_that/) | 竞品作者 | 表明已有浏览器截图前自动遮挡产品在尝试此问题 | **不能**计为用户需求 |
| P05 | 2026-08-12 [PrivacyCam 开发者推广](https://www.reddit.com/r/SideProject/comments/1vmc8wz/i_built_an_ondevice_app_for_hiding_private/) | 竞品作者 | 发现已有多格式本地脱敏工具供给 | **不能**计为用户需求 |

本步骤满足“至少两条有日期、可追溯到不同公开提问者的相关问题表达”的低门槛（P01/P02）。**不**声称两人都是“重复打码的重度用户”，不引用任何私人联系方式，帖子仍可能有选择偏差或删除风险。部分 r/Compliance 回复标注供应商从业者身份，应与 P01 的原始问题分开。

## 2. 招募可行性——渠道调查结果

| 途径 | 已核对机制 / 官方链接 | 使用前必须满足的条件 | 当前真实状态 |
|---|---|---|---|
| [Prolific](https://www.prolific.com/how-it-works) | 官方支持研究者创建研究、受众筛选、连接外部实验、预存研究经费；[价格](https://www.prolific.com/pricing)列出按人付费加平台费用 | 研究者账号、余额、筛选规则、合规同意、研究页面和目标语言参与者容量 | **有理论执行渠道；账号/预算/实际可招到人数 NOT_VERIFIED** |
| [User Interviews](https://www.userinterviews.com/recruit) | 官方提供研究参与者筛选、排期与激励；[按场价格](https://www.userinterviews.com/pricing)说明会话招募收费且激励另外计算 | 研究账号、资金、样本筛选及受试者同意 | **可选但成本较高；未开通/未支付/未招募** |
| 社区自愿报名 | 可考虑先征得社区管理员允许后发布非诱导招募公告 | 具体论坛招募政策、管理员许可、隐私说明、报名/排期通道 | **尚未获社区许可；不得当作现成渠道** |
| 已有个人联系网络 | 可能用于定性探索但存在熟人偏差 | 用户真实自愿同意，不能公开身份或上传真实图片 | **本会话无被试名单、不能由 AI 代招** |

**操作性结论：`RECRUITMENT_PATH_CONDITIONAL`。** 市场有真实招募服务，但本研究未配置研究者账号、预算、招募公告或外部调查页面；目前 **无法确认能否招到符合条件的 10 人**。这是一项**明确已记录的可行性限制**，不是项目需求假设 A1 获证。

## 3. S01 验收逐条判断

1. 两条以上日期明确、不同发言主体的真实公开问题：**满足（P01、P02）**，但均不是受试者样本或行为测量。
2. 排除开发者广告/推广充当用户研究：**满足（P04、P05）**。
3. 对招募是否可行做实地可审查判断：**完成桌面渠道调查；真实招募能力为 CONDITIONAL / NOT_VERIFIED**。不得升级为 `RECRUITED`。
4. 对缺失与非目标行为留痕：**满足**；未进行联络、报名、同意或付费。
5. S01 的 `test -s` 只检查文件存在；**不能证明内容实质有效**，需要同一 Agent 内部审查及后续独立审查。

建议把**S01“最小问题线索与招募可行性评估”记录为 DONE（仅这一执行任务）**，并在后续 S04/S05 保留所有实际招募阻断。不要由此推出 A1 通过或市场验证成功。

## 4. 研究结果与下一任务

- `public_problem_posts_qualified=2`（仅广义相关且强度不同；不代表被试或统计样本）
- `recruited=0; consented=0; user_sessions=0; comparative_task_runs=0`
- `A1=OPEN / frequency=NOT_VERIFIED / switching_intent=NOT_VERIFIED`
- `recruitment_operational_state=CONDITIONAL_NOT_VERIFIED`
- 下一步骤 S02：四种对照工具的版本/能力矩阵与 **五套真正生成的合成测试 PNG/JPEG + 可重算 JSON 真值**；不得把“规范存在”当素材文件已经产生。
- 凡原资料后来被撤稿/不可访问，应作为证据退化重新审核；不悄悄换掉数据来源。

**审查身份：原执行 Agent 自审，不宣称第三方独立验收。Stage 01 终局必须单独独立审查。**
