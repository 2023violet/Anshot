# E1 独立技术验证｜方案与授权边界

**2026-10-15 · APPROVAL REQUEST — PREVIEW ONLY**。

## 为什么不直接“replan”旧 Stage 01

原 [Gate A 决策卡](../decisions/STAGE01_GATE_A_CARD_READY.md) 把 Rust 应用代码明确列为非目标，原 [Gate B 九步计划](../plans/STAGE01_GATE_B_MACHINE_INPUT.json) 的 S06 依赖 S05a/S05b 的真实用户证据。当前 S05a = BLOCKED_BY_RECRUITMENT，A1 市场需求未知。如果直接删掉 S05 的依赖或把 E1 编码当作 Stage 01 已批准工作，既会违反既有授权，也会破坏审计线索。

**方案：为 E1 建立独立 Skill 决策工作区，原有 Stage 01 的 Gate A/B 和机器状态保持原样。** 这样可在技术层验证“值得试一个最小 Rust/WASM”而不捏造十名用户研究。新 E1 仍需经过自己的 Gate A、Gate B 两次独立人工确认，以及明确的代码 SPIKE 授权；研究项目依旧保留阻断。

## 事实、选择和授权范围

| 选择 | 说明 |
|---|---|
| A — 本次选择 | 独立、可撤回的 E1 实验分支。先 Gate A 预览，不确认；之后 Gate B 预览，不确认；两轮得到人类许可后才开始非常小的 Rust 测试代码 |
| B — 不选择 | 直接修改 Stage 01 原 gate B，跳过真人研究依赖并宣称 S06 PRD 冻结；缺乏市场证据且改变初次授权范围 |
| C — 不选择 | 现在全量实现 UI + Rust 功能；风险太大，尚未证明安全核心与 clipboard 兼容性 |

**原 Stage01 真实状态**：Gate A/B PASS；S01–S04 DONE；S05a BLOCKED_BY_RECRUITMENT；原研究 A1 未验证。原始决策卡 blob e542ef0745a60a3f439979fe45df92cf059b37c6，原始 machine plan blob d24b33c5c77d13a3a1efc67c5eb6a7b63e7986bc。

## E1 验证对象

决策卡：[E1 Gate A 候选](../decisions/SHAREREADY_E1_GATE_A_CANDIDATE.md)。待 Gate A 人工批准后才能正式产生 Gate B token。六步执行计划：[E1 Gate B 候选（未批准）](../plans/SHAREREADY_E1_GATE_B_CANDIDATE.json)。该 JSON 内的 `verify_cmd` 将在未来执行各阶段测试，**本次不能执行，也不能因为 CLI plan check 通过而启动代码**。

必须提交：决策卡目标/成功标准/止损、计划更改内容、工作区 fingerprint、每一步预期操作与副作用、每项安全检查的真实机器证据。

## 此次授权的准确解释

用户“进入下一步”= 允许**准备并展示真实 Skill 新 Gate A 预览**，不等于对预览 token 的确认，不等于 Gate B 的确认，更不等于授权 Rust 编码。

**下一步将真实运行固定 Skill `4.1.0`（upstream Git SHA `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`）在隔离工作区执行 `init` / `check card` / `check plan` / `gate A`（不带 `--confirm`）。预览必须经人类回复批准才运行 `gate A --confirm`；之后再审阅 Gate B 的 `verify_cmd`。**

不更改 main、旧 .decision、用户私人数据、部署、发布和旧 PR 审批状态。
