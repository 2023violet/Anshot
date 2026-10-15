# SR-RES-010｜S04 审查与招募可行性复核

2026-10-15 · SAME-AGENT ADVERSARIAL DESK REVIEW · EXTERNAL HUMAN REVIEW PENDING

**本报告不是独立人类伦理批准、法律意见或对外招募授权。** 目前：招募公告 0、实际联系 0、报名 0、同意 0、访谈 0、付款 0、A1 假设 OPEN。

## 1. 审查依据与纠偏

审查了 Stage 01 Gate B 的 S04 原始任务标准、STAGE01_PARTICIPANT_STUDY_PACK.md、S04_RESEARCH_PROTOCOL.md、S04_CONSENT_AND_SCREENERS.md、10 个未分配槽位及 S03 已完成的真实竞品试验。

| 风险 | 审查发现 | 修正/阻断 |
|---|---|---|
| R01 CRITICAL | 研究主体、联系人未确定 | 必须在招募前填写真实研究负责人和保密联系渠道 |
| R02 CRITICAL | 伦理上同意参与≠个人数据处理的法律依据 | 分开写；适用法域与具体依据留给实际运营方核实 |
| R03 CRITICAL | 撤回数据与销毁期限过于笼统 | 参与可随时中止；可关联数据撤回方式、时间及匿名化后局限须事前告知 |
| R04 CRITICAL | 私有联系方式存储、访问控制、删除责任人未知 | 禁止把联系资料、映射表、录音、IP 或自由文本原记录推到 GitHub |
| R05 HIGH | 真实在线竞品可能有遥测与第三方请求 | 必须披露第三方外部网站；只能提交合成图片，不能保证工具绝对无网络流量 |
| R06 HIGH | 只招有打码经历的人会人为制造产品需求 | 纳入最近 30 天零需求者并保留反证；记录招募拒绝与退出 |
| R07 HIGH | 固定 AB/BA 十槽不是已经随机分配的受试者 | 设计已平衡，真正招募后在私有系统用安全随机方式从未用槽位选择 |
| R08 HIGH | T01 与 T02 难度不同，顺序平衡不能自动消除差异 | 工具与任务分层，不用未经测量的因果优势或统计显著性结论 |
| R09 HIGH | 录音/屏幕录像易与基本任务同意捆绑 | 三项分开；录音、录像缺省不同意，拒绝不影响无录制研究 |
| R10 HIGH | AI 操作计时可能被误解为用户操作 | 严禁拿 S03 的预知坐标 Playwright 时间作真人效率数据 |
| R11 HIGH | “已经下载”不等于像素安全 | 对真实导出字节和敏感/保留真值比对，保存失败与漏遮挡 |
| R12 HIGH | 第三方访问可能要求账号 | 独立无登录浏览器；如需私人账户立即停止 |
| R13 MEDIUM | 十人不足以估算市场普及率 | 定性机会研究；允许负面结果、PIVOT 与 NO-GO |
| R14 CRITICAL | 研究者平台/预算/法律地区均未验证 | 不外联、不招募、不支付、不自行确认已具备运行条件 |
| R15 HIGH | 只有同一 AI 的审查，没有独立真人伦理或隐私复核 | 标识 EXTERNAL_REVIEW_PENDING，绝不声称 ETHICS_APPROVED |

## 2. 外部参考资料及使用限制

- [ICO: Research, lawful basis and consent](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/the-research-provisions/principles-and-grounds-for-processing/) 解释“参与研究的同意”与“个人信息处理的法律依据”不同。
- [ICO: Privacy notice content](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/the-right-to-be-informed/what-privacy-information-should-we-provide/) 要求明确组织/联系人、目的、期限、适当的权利和处理情况。
- [ICO: Research safeguards](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/the-research-provisions/what-are-the-appropriate-safeguards/) 强调数据最小化；假名化仍可能构成个人数据。
- [Prolific current pricing](https://researcher-help.prolific.com/en/articles/445239-what-is-your-pricing) 列出参与者最低报酬 $8/h、推荐 $12/h；一般企业平台费用约为奖励的 42.857%，税和筛选费用另计。
- [Prolific payment policy](https://researcher-help.prolific.com/en/articles/445230-prolific-s-payment-model) 指明研究奖励、筛选补偿等规则。
- [Nielsen Norman Group on usability sample sizes](https://www.nngroup.com/articles/how-many-test-users/) 可参考定性发现实践，但不能证明本产品需求普及率。

ICO 主要是英国监管背景。实际研究主体/参与者所在法域仍 UNKNOWN，不能把引用资料等同于本项目的合法性批准。

## 3. 招募平台与费用的纸面可行性（没有付款或订单）

| 路线 | 官方来源 | 本轮结论 |
|---|---|---|
| Prolific | https://researcher-help.prolific.com/en/articles/445239-what-is-your-pricing | 具备有偿样本渠道；账户、目标语言和地区容量、外部同步访谈能力、结算方式均未验证 |
| User Interviews | https://www.userinterviews.com/recruit | 有招募/筛选服务；真实报价与账号权限未验证 |
| 社区自愿报名 | 各社区版规须另外核实 | 未获版主许可，未发送消息，不假定可以直接招募 |

**预算模型，仅假设：** 10 人 × 30 分钟 × $12/h = $60 参与报酬，按企业平台费粗算总计约 **$85.71**，未计税、筛选、加时和其他系统费用。45 分钟场次对应 $90 奖励，含同等平台费约 **$128.57**。不是平台报价、已审批预算或实际支出。开源 MIT 许可证不代表自动具备平台非营利折扣资格。

## 4. 门禁界限

**S04 文档准备与审查门禁：** 审阅中立筛选、知情同意草稿、联系数据隔离、AB/BA 平衡任务、真实计数，出具上述问题清单并留痕；达到原 Gate B 的任务级文件要求，可由固定 Skill 独立验证记录后标为完成。

**对外招募门禁：STOP_RECRUIT。** 在下列事项由真实运营者全部确认前不得发布公告或接触参与者：负责人、保密联系方式、适用地区与个人数据处理依据、存储与销毁流程、激励/预算决策、第三方服务和录制告知、独立人类伦理/隐私审查。S05 是真实参与者步骤，不能用文档替代同意或访谈。

审查性质：SAME_AGENT_DOCUMENT_REVIEW, NOT_EXTERNAL_ETHICS_APPROVAL。研究人数和付款均为 0；A1 仍未验证；无 Rust 产品代码、合并或发布。
