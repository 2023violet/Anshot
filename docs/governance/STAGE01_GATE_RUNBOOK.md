# ShareReady Stage 01｜真实 Skill Gate A/B 执行 Runbook

**SR-GOV-002 · 日期：2026-10-14 · 状态：PROCEDURE WRITTEN / NOT EXECUTED**

## 结论与阻断原因

2026-10-14 用户已批准 Stage 01 的**人类决策候选**，见 [批准记录](../decisions/STAGE01_GATE_A_APPROVAL_RECORD.md)。但执行环境内 `git ls-remote https://github.com/2023violet/ShareReady.git HEAD` 与 Skill 仓库的同类请求均因 `Could not resolve host: github.com` 失败；GitHub 连接器可以读写文件，却不能执行 `de.py`。

因此：
- `GATE_A_CANDIDATE_HUMAN_APPROVED = true`
- `GATE_A_SCRIPT_PASS = NOT_VERIFIED`
- `GATE_B_SCRIPT_PASS = NOT_VERIFIED`
- **不手工伪造** `.decision/plan.json`、`.decision/events.jsonl` 或 token 审批事件。
- PRD/研究计划只能作为待审草稿放在 draft PR；所有实验、Rust 代码与发布动作继续受限。

## 版本与授权边界

- ShareReady 当前工作分支：`docs/stage01-direction-skill-gates`，基线来自提交 `430eab847a315b07480dda3e50eac4493759f416`，执行前必须重新检查实际 HEAD。
- 技能源：[deciding-and-executing](https://github.com/2023violet/deciding-and-executing/tree/1eb9624ee2037e4a2b253ee8120fa274dad1ef21) SHA `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`，元数据 `4.1.0`（candidate）；配套 closing-out 同源。
- 原文政策：[SKILL.md](https://github.com/2023violet/deciding-and-executing/blob/1eb9624ee2037e4a2b253ee8120fa274dad1ef21/skills/deciding-and-executing/SKILL.md)。应使用源代码本体，而不是复制 README 伪实现“同等门禁”。
- 未经重新确认不得切到以后版本；不得以 token 替代真实的人类同意。

## A. 在能访问 GitHub 的**独立本地环境**准备（示意命令，未执行）

以下示例在 Bash 中演示；Windows PowerShell 可使用等价的 `git clone` 和 `py -3`。**若已存在本地仓库或 .decision 工作区，请先 `status`，不得删除、覆盖或重新 init。**

```bash
git clone --branch docs/stage01-direction-skill-gates https://github.com/2023violet/ShareReady.git
git clone https://github.com/2023violet/deciding-and-executing.git
git -C deciding-and-executing checkout --detach 1eb9624ee2037e4a2b253ee8120fa274dad1ef21
cd ShareReady
git status --short
git rev-parse HEAD
```

如上出现异常分支、dirty tree、未授权修改，立即停止、记录实际状态，不清理用户数据。

在相邻目录安装/保留完整 upstream skill 后，下列命令将调用真实 `de.py`。路径仅为演示，依实际本机布局调整。

```bash
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py init --name shareready-stage01 --risk standard
cp docs/decisions/STAGE01_GATE_A_CARD_READY.md .decision/decision-card.md
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py check card
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py status
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py gate A
```

**必须停在这里**：完整展示真实返回的 `preview`（目标、编号成功标准、关键假设、止损）与 `token`，请用户在**下一条独立回复**确认后再执行：

```bash
# 仅在针对真实 preview 获得独立同意后：
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py gate A --confirm <ACTUAL_TOKEN_FROM_PREVIEW>
```

如 `check card` 失败，仅修复格式或有证据的问题；如果修改了用户批准的目标、成功标准、非目标或止损条件，需重新呈交批准。若脚本接口与此示例不同，以固定 SHA 的 `--help` 和实际报错为准，不绕过。

## B. 进入计划阶段及 Gate B（只有 Gate A 成功后）

1. 以 [Gate B 草案](../plans/STAGE01_GATE_B_CANDIDATE.md) 拆分实际 `.decision/plan.json`（要使用真实 `init` 生成的 schema，不要直接粘贴不合法 JSON）。
2. 先把 A1 的测试放在最前，把标准 1–5 的覆盖写入 `covers`；实验步骤用 **真实材料文件** 证明，不用假 `verify_cmd` 装作跑了用户研究。
3. 执行 `check plan`、`show`、`preflight`，并检查将要执行的 shell 命令/副作用；然后 `gate B`。
4. 展示脚本返回的 Gate B 真正的 `preview`、验证命令与 token；等待用户下一条明确同意。
5. 只有独立同意后，才执行真实 `gate B --confirm <ACTUAL_TOKEN>`。

```bash
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py check plan
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py show
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py preflight
python3 ../deciding-and-executing/skills/deciding-and-executing/scripts/de.py gate B
# STOP: present preview to human. Never self-confirm.
```

## C. 证据规则

- `COMMAND_PASS` 仅意味着对应命令在指定代码指纹上成功，不能自动代表 UI、安全、用户研究成功。
- `ARTIFACT_INTACT` 只意味着文件还在、内容哈希未改变，不意味着访谈内容真实。
- `coverage` 只列成功标准/步骤关联，不能代替独立验收。
- CLI 或真实用户测试缺失必须保留 `NOT_VERIFIED`；远端 GitHub 文档提交 **不是** Gate A/B 成功。
- 工作区中不得放真实私人截图；不要随意执行 `purge`、覆盖已有文件、改权限或合并/发布。
- 里程碑结束使用同源的 `closing-out`，分清静态审计、事实校验、代码运行和人工测试层。

## 当前状态

**`BLOCKED_BY_ENV / HUMAN_CANDIDATE_APPROVED / SCRIPT_GATE_A_PENDING / GATE_B_PENDING`**
