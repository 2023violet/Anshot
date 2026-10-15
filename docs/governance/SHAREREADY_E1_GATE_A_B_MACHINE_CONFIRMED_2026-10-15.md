# E1｜Gate A / Gate B 真实机器确认报告

**2026-10-15 · BOTH_GATES_VALID / E1_STEPS_0_OF_6 / PRODUCT_RUST_CODE_NOT_AUTHORIZED**

## 精确授权范围

用户明确批准 token `b64d6bae91`，要求重新核验原计划、固定 Skill、六条验证命令和 token，并执行真实 `gate B --confirm`，同时明确**不授权 Rust 编码、PR 合并、部署或发布**。严格按此界限完成。

## 实际执行证据

- [GitHub Actions Run #38747997797](https://github.com/2023violet/ShareReady/actions/runs/38747997797)：**SUCCESS**，runner HEAD `d63bba74a4a40396a6404962ede145a6e436fefd`。
- 运行 Skill：作者仓库 `2023violet/deciding-and-executing`，固定 commit `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`，版本 `v4.1.0`。
- 原 Stage01 卡片 blob `e542ef0745a60a3f439979fe45df92cf059b37c6`、原 Gate B 计划 blob `d24b33c5c77d13a3a1efc67c5eb6a7b63e7986bc` 均逐一核查不变。
- E1 Gate A 卡片 blob `d4687fed6a58bf17ac2505c5bfb599d2ab5f205d`、E1 六步 Gate B 计划 blob `f008a25717a6623186dca2675d322ce2049d09dc` 均逐一核查不变。
- 恢复真正 [Run #38745548101](https://github.com/2023violet/ShareReady/actions/runs/38745548101) 的“Gate A valid / Gate B missing”机器状态工件，内部 tar SHA-256 `e9dd85513c0e0d3db1d120ca18125b77fa5ae4cecbb7193da5009e68f4b6fb4c` 校验通过。
- 重新调用 `de.py check card`、`de.py check plan`：0 error / 0 warning。
- 重新调用 **实际 `de.py gate B` preview**：正确返回需要确认，token 仍为 **`b64d6bae91`**，包含全部六条验证命令的 `preview.steps` 与上次工件中保存的整个 preview JSON 内容一致；未发生计划漂移。
- 执行真正的 `de.py gate B --confirm b64d6bae91`，返回 `{"ok":true,"gate":"B","first_step":1}`。
- 最终 `de.py status` 为 `gates.A=valid`、`gates.B=valid`；计划全部六步的 `status=todo`，未有任何已执行 E1 步骤。
- [机器档案 Artifact #11979962656](https://github.com/2023violet/ShareReady/actions/runs/38747997797/artifacts/11979962656)，GitHub ZIP sha256 `6511729419e7cd9b359a78bc47666dd4a994e2662af86ec4e33c75f2118b8f28`；内部 `.decision` tar SHA256 `e7ce9925fc5ea3ff1316dab1bed62a3bf03f173edded1176d5d332a4eb6c9dc8`，预计 **2026-11-14T14:03:33Z** 到期。

## 额外防错记录

首次本轮尝试 [Run #38747782425](https://github.com/2023violet/ShareReady/actions/runs/38747782425) 因辅助校验脚本错误地读取了不存在的 `preview.verify_commands` 字段而在 **确认之前**中止；此时重新计算的 token 与实际预览已经一致，**并未运行 `--confirm`**。更正为原版 Skill 实际字段 `preview.steps` 后，重新从原始存档启动全新 runner，并通过全部检查，见成功 Run #38747997797。不存在重复 B 确认或跳过审核。

## 六步已批准但尚未执行的计划

| 步骤 | 计划 | 当前真实状态 |
|---|---|---|
| 1 E0 | 独立分支/工具链、实施授权与源码基线预检 | TODO |
| 2 E1a | Rust 几何像素实心遮挡、合成测试 | TODO |
| 3 E1b | 全新 PNG 重编码、元数据与错误输入测试 | TODO |
| 4 E1c | Chromium Rust/WASM 本地导出与真实文件复核 | TODO |
| 5 E1d | 复制权限、下载回退、内存及网络观察 | TODO |
| 6 E1e | 独立安全/架构审查与最终技术决策 | TODO |

## 双重治理状态

**独立 E1**：Gate A PASS、Gate B PASS；六步未开始。确认 Gate B 并**不**代表已获代码执行授权：在下一个用户授权前不得创建/修改 Rust 产品代码、构建完整工作区或运行 E1 测试命令。未来任何计划文本改变都必须重新计算并按 Skill 重新走门禁。

**原 Stage 01**：批准的研究计划、已执行的状态与阻断均未被改变：S01–S04 DONE、S05a BLOCKED_BY_RECRUITMENT、真实参与者 0、A1 用户价值 OPEN。

**禁令保持**：无 Rust 编码、无 `main` 修改、无合并、无强推/改历史、无部署、无 tag、无 Release/发布。
