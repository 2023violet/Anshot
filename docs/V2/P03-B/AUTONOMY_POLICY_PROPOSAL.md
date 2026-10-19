# 连续自主开发授权提案（尚未生效）

本提案用于落实用户“按照计划一直推进直到完成完整的产品”和减少人工介入的要求。它尚未修改 AGENTS.md，也未替代现有闸门规则；需要用户明确批准后生效。

## 当前冲突及最小调整

当前 AGENTS.md 的明确要求是：**“Gate A/B previews require separate human acknowledgment. Never self-confirm a token in the same exchange.”** 作者工作流也规定生成预览后等待用户同意；阶段审查和归档同样包含人工确认。该规则要求每个新批次等待，现有“继续执行”的授权不能由代理静默改写成某个未展示 token 的人工确认。

建议仅为已批准总方案内的可逆本地开发设置下列例外。保留所有技术判据、风险分类、真实 CLI、证据、停止条件和审查；改变的是普通本地批次由谁确认，不能把代理确认记成人工验收。

## 拟加入 AGENTS.md 的生效条款

> Continuous local development authorization: when the user explicitly approves this policy, the agent may autonomously plan, implement, test and review reversible local batches within the approved V2 product roadmap. For these batches only, the agent may generate and confirm genuine workflow Gate A/B and review tokens, and apply non-destructive documentation archives, without another per-token human reply. This is an explicit exception to the human-acknowledgment requirements of this AGENTS.md and the adopted author workflow for that bounded scope. Record the policy approval and each agent confirmation honestly; never describe an agent-confirmed token as a human-reviewed token.
>
> Retain main-only development, expected-HEAD checks, preservation of unrelated work, Rust safety-critical transformations/export, synthetic fixtures, locked dependencies, acceptance coverage and exact-artifact evidence. Production-impacting local code changes retain high-risk evidence and independent review requirements; the exception only removes repeated human confirmation for reversible local work. Independent review may use a separate sub-agent with the change and acceptance criteria; do not treat the implementer's own review as independent.
>
> The exception does not authorize deleting/moving user files or the V1 path, rewriting history, force-pushing, changing secrets/authentication/permissions/security or privacy settings, installing drivers, purchases, external messaging, uploading user data, remote publishing, pushing or releasing. Surface the concrete impact and obtain express human confirmation before such actions. Never silently relax success criteria, replace frozen goldens, claim unobserved human usability/accessibility tests passed, or claim the full product complete from partial machine tests. Preserve unresolved environment and human-evidence gaps; continue independent authorized work where possible.

生效时对原 Gate A/B 条款加上“上述连续本地开发例外除外”的明确交叉引用，避免两条规则互相矛盾。保留固定作者版本，不篡改上游脚本。普通批次的 CLI token 确认属于代理确认，用户批准的是这份明确的执行政策，而非未来每个预览的逐字人工审阅。

## 批准后先执行什么

1. 将本条款写入 AGENTS.md 并记录用户授权；不改系统权限或 Codex 安全设置。
2. 基于已取得的 B0 原始证据结束基线分类。优先建立 M01 取证/独立 PNG oracle，再按 D02 的物理像素语义补 M02 和 M03 必需入口。Ctrl+Z 未产生可观察变化与撤销后陈旧状态文案需先复现、再有界修复。
3. 对每个实际代码批次形成真实决策卡和命令计划，以合成反例、Rust 回归、原生自动操作和独立审查验证。超出本地可逆范围、出现泄露/陈旧成品成功/静默丢编辑或无法满足必要验收时停止受影响路径并报告。
4. 自动操作尽量替代人工代测；工具无法观察的像素、跨屏、音频或人类体验继续明确标为 NOT_VERIFIED，不伪造结论。普通功能实现和可独立执行的测试继续推进。

## 不批准时的行为

保持 AGENTS.md 原规则不变。已确认的 B0 测试继续有效；新批次仍按原规范展示并等待各次闸门确认。本提案本身不批准产品发布、不改变品牌未清查状态，也不宣布 GUI 选型或 V2 完成。
