# SR V2 03 B0：基线收集完成，验收缺口明确

日期：2026年10月17日。B0 基线收集与分类完成，已生成“止损”归档，停止继续扩展本轮有限观察；这不表示放弃产品或判定框架失败。历史两条链接已修正，genuine CLI 完整知识同步通过；工作区和原始证据保留。**本页及其中的确认记录只描述原固定 EXE；当前代码与下一批证据见 [P03-C](../P03-C/README.md)。G01～G10 完整验收仍未通过。**

## 当前结果

[本机结果与 G01～G10 判定](B0_RESULTS.md)：33 项 Rust 测试和全目标检查通过，EXE 自检和 DX12 窗口通过，实际适配器为 GTX 1650。已通过自动键盘与无障碍树验证部分复核/历史/交付状态；双屏指针、DPI 和 Narrator 仍未验证。

## 已完成的准备

- genuine 作者 CLI 已确认上一轮展示的 Gate A；[原始确认返回](GATE_A_CONFIRMATION.json) 记录成功，初始化计数警告已消失。冻结决策卡未改。用户随后确认了已展示的 Gate B，见 [真实确认返回](GATE_B_CONFIRMATION.json)。
- 实际五步计划已运行 genuine `check plan`：错误 0、警告 0。`preflight` 另提醒工作区有文档改动，归属已核对，不自动清理。
- 源码固定为 `94f7f419aee91faf6f846349d40a897d0d72d7d0`；EXE SHA256 为 `386200cc23ff8015258b655a6911c1993c8d08e8fef3e57ca826e9757fc76b9d`。每个机器步骤重新核对。
- 本机只读查询为 rustc/cargo 1.98.1，MSVC Build Tools 存在。待测 EXE 的 CI 工具链为 1.99.0，本机回归单独记证；本机 workspace 编译检查已经通过；未重建 POC EXE。
- 已准备 [机器执行辅助脚本](commands/run_b0.py) 与 [人工观察单](HUMAN_OBSERVATION.md)。脚本 identity/probe/regression 三种模式均已实际通过；交互样机已由 computer-use 实际操作，原始状态见 B0_AUTOMATION_OBSERVATIONS.json；人工使用性证据未伪造。

## 本次 Gate B 五步

| 步骤 | 动作与证据 | 预计投入 |
|---|---|---|
| B0.1 | 核对 main/HEAD、EXE、18 个来源文件、五张输入、操作单及工具哈希；记录工具版本与工作区差异 | 0.5h |
| B0.2 | 固定 EXE `--self-test`；`--renderer wgpu --smoke-window`，子进程限定 `WGPU_BACKEND=dx12`；记录退出码、窗口标记与实际适配器。随后先做人工单第 1 项最小 H01 观察 | 1h |
| B0.3 | 本机固定 Cargo/Rust 执行 `cargo test --workspace --locked` 和 `cargo check --workspace --all-targets --locked`；不构建新 POC | 1h |
| B0.4 | 现有双屏设置下观察指针、缩放、键盘及 Narrator；真实操作者报告与合成截图留证 | 3h |
| B0.5 | 逐项映射 G01～G10，分类实现、环境和取证缺口，提出下一批最小修复与复验条件；不实施产品修复 | 1h |

依赖为 1 → 2；3 和 4 依赖 2；5 依赖 3 与 4。用户随后明确授权代理控制本机，第 4 步以自动操作取得有限状态证据，人工使用性、截图和多屏等缺口保留 NOT_VERIFIED。投入为预计工程/操作者时间，不是交付承诺。

## 绑定命令与副作用

三个真实 `verify_cmd` 调用已绑定 Python 脚本，模式为 `identity`、`probe`、`regression`，参数包含本机绝对路径与上下文哈希。完整原始 CLI 预览、计划及上下文保留于本次任务本地 `workflow-gatea/`，用于检查实际命令。仓库内 [Gate B 预览](GATE_B_PREVIEW_REDACTED.json) 和 [计划](EXECUTION_PLAN_REDACTED.json) 明确标为确认前的路径脱敏副本，不冒充原始 stdout，不可直接执行；当前步骤状态以 CURRENT_STATE.json 和 genuine 工作区为准。

- 上下文 SHA256：`7cdec5e579a863beb5460f6094f0a30b8ff39dec03da031861ec63c58f28a379`。
- 辅助脚本 SHA256：`67d3cf6713a2510063c7aad59418fd14aed9c88789a34f10757332aaef0218bf`。
- 新日志与人工证据写入已忽略目录 `target/p03-b-evidence/b0-20261017/`；每次机器执行新建唯一子目录，不覆盖或删除旧日志。Rust 编译写入既有 `target/` 构建缓存。
- Cargo 缓存不足时可能从已锁定来源联网下载依赖，并写 Cargo 用户缓存。不升级锁文件、不安装工具链、不添加依赖；不向网络发送图片。
- EXE 自检及窗口探测各有 60 秒超时；超时仅终止本次新建子进程。每条 Cargo 命令最多 1200 秒。失败保留日志，不自动改产品或清理缓存。
- 窗口探测会短暂显示 GUI；人工观察另启动固定 EXE（`--renderer wgpu`），由操作者关闭。只使用合成图，只读查看显示设置、移动窗口，不改分辨率、DPI、驱动、权限或防火墙。
- 人工观察允许临时切换 Narrator 并恢复原开关。不操作剪贴板、不保存或覆盖图片。截图只采集合成画面，由操作者确认后放入证据目录。
- 不删除、提交、推送或发布；保留 V1、金样本、总方案、PDF 和 P03 A 封存包。M01～M09、D02～D05 合同变更及框架冻结不在本次范围。

命令风险提示为空不等于没有副作用。本机真实执行结果与日志哈希见 B0_RESULTS.json，测试边界单独保留。

## Gate B 预览及确认历史

此前真实 `gate B` 预览返回退出码 3、`needs_confirmation=true`，token 为 **`458cf737c7`**。用户看到预览后明确确认，已通过 genuine `--confirm` 完成确认，当前 Gate B valid。原始预览保留为历史。

确认即批准上表 B0 命令及明确列出的可逆操作。步骤、命令、上下文或辅助脚本改变后，重新绑定并生成预览，不能沿用旧确认。本次用户已完成单独确认，无需重复确认同一计划。

## 证据边界与历史

窗口生命周期、自检的 Rust 对照和 Cargo 通过，均不等于独立像素 oracle、真实指针映射、混合 DPI、完整键盘或 Narrator 通过。实际物理适配器选择另有进程日志，仍不证明性能。P03 A 的 33 用例仍为规格；无人操作不得代填。Fit、原像素复核、粘贴和完整键盘入口的缺口单独记录，不放宽原合同。

- [冻结决策卡](GATE_A_DECISION_CARD.md) 与 genuine 卡字节一致。
- [历史 Gate A 预览](GATE_A_PREVIEW.json)、[卡检查](CHECK_CARD.json)、[历史来源摘录](WORKFLOW_PROVENANCE.json) 保留原始时点的状态和初始化警告，不代表当前状态。
- [计划检查](CHECK_PLAN.json) 是原始 CLI 输出；[当前状态](CURRENT_STATE.json) 记录 A/B 已确认、基线收集与分类完成及验收缺口。
- [P03 A 包](../P03-A/README.md) 保留初始化前的准备记录，未重写历史校验前提。
- `SHA256SUMS` 校验本目录文件，不代表产品或人工验收；独立审查尚未完成。

工作流仍固定作者提交 `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`、4.1.0 candidate。下一步是依据已取得证据进入有界产品修复。为落实持续自主开发，参见 AUTONOMY_POLICY_PROPOSAL.md（待批准，尚未生效）；未批准 V2 发布。
