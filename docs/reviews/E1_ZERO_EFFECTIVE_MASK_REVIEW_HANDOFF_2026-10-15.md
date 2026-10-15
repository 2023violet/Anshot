# SR-E1-REVIEW-011｜零有效遮挡导出边界修复：独立审查资料包

> **2026-10-15 · CODE_FIXED / RED_GREEN_VERIFIED / ORIGINAL_E1_CI_REVERIFIED / INDEPENDENT_REVIEW_PENDING**
>
> 本文档是审查交接与机器证据索引，**不是独立审核者的签署或 E1e PASS**。不可用本文件冒充 `docs/reviews/E1_SPIKE_INDEPENDENT_REVIEW.md`。实际 Rust 技术实验已由用户授权，但主分支合并、部署与正式发布不属于此次操作。

## 1. 审查对象和固定源码

- [Draft PR #10](https://github.com/2023violet/ShareReady/pull/10)：工程分支 `spike/e1-rust-wasm-feasibility` → 研究分支 `docs/stage01-direction-skill-gates`，未合并。
- 原有被审查提交：`8972a924b256cb065f91b3ab227391aa4405da95`。
- 完成定向代码修复提交：`cf2315b9546bd7bf6212d5c9a283fb9b84226f10`，Rust `e1-spike/src/lib.rs` git blob `ab4871a663b210076fe70711adcee1451dd33cbf`。
- 原版 Rust 核心复验提交：`7ef598cfcdec10a574c2724c6a7857bbf4937939`。
- 原版 Rust/WASM 浏览器复验提交：`862c13f509236d895f260fdb81d20db06887752a`。
- 影响范围严格为 `e1-spike/src/lib.rs`、`e1-spike/tests/codec_security.rs`、`e1-spike/tests/verify_browser_export.py` 和三条 CI 工作流；未修改 Cargo.lock、原 Skill Gate A/B 输入或正式 ShareReady 产品架构。
- 原 Skill pinned SHA：`1eb9624ee2037e4a2b253ee8120fa274dad1ef21`，原 E1 六步计划仍不变。

## 2. 缺陷及修复

原 `redact(input, regions)` 会在 `regions=[]` 或所有矩形经半开像素坐标裁剪后面积均为零的情况下，把**未遮挡源图重新编码为一个看似有效的 PNG**。同样的 `redact_web` WASM 公共入口接受空数组。

**修复：** 当矩形列表为空时立即返回错误；否则在成功解码/归一化后的真实像素 `width,height` 上逐个检查 `Region::clipped`。如果不存在至少一个有效的非零面积裁剪矩形，返回错误 `no effective redaction region; refusing unmasked PNG export`，`paint_rgba`、PNG 编码与成功导出路径均不执行。**混合无效矩形及有效 1×1 遮挡仍可输出。**

本修复只保证**非零遮挡覆盖范围**；它不能检测用户遗漏了哪些敏感内容，也不能确认遮挡区域是否覆盖了所有应保护信息。

## 3. 同一测试的 RED → GREEN 机器证据

| 检查 | 修复前基线（预期失败） | 修复后（实际成功） |
|---|---|---|
| 新增 Rust 集成测试的无效遮挡集合（空列表、越界、零宽/高、边界相切、极大坐标） | [Run #38757327352](https://github.com/2023violet/ShareReady/actions/runs/38757327352)：`rejects_empty_or_ineffective_masks_without_exporting_source` FAIL，测试 5 passed / 1 failed | [Run #38757489210](https://github.com/2023violet/ShareReady/actions/runs/38757489210)：6 passed / 0 failed，原生两轮检查均 PASS |
| WASM 真正实例上的五种负向 API 输入 | 同一 RED Run：五种均实际错误地返回 PNG bytes | 同一 GREEN Run：五种均返回 fail-closed 错误且没有输出 bytes |
| 正常路径与不兼容矩形混合输入 | 原正向基线有效 | GREEN Run Rust 1×1 有效遮挡和保留周边像素测试通过 |
| 合成浏览器导出 | 无效输入时有错误成功路径 | GREEN Run 四份 T01/T02/T04/T05 原样保持真实 PNG 下载、源像素全框遮挡与指定保留区检查通过 |
| Clipboard / 浏览器网络 | 先前实验仅支持观察结论 | GREEN Run 记录写入成功但未完全回读、拒绝及 PNG 下载替代；没有声称跨浏览器认证 |

**新增回归工件：** [Artifact #11984471355](https://github.com/2023violet/ShareReady/actions/runs/38757489210/artifacts/11984471355)，GitHub ZIP sha256 `4a0c9336cd7dd60e23c01d33ad6f9a840d95069adab15966a7ac60f281278e39`，预计 2026-11-14 到期。包含真实导出 PNG 及浏览器会话指标。

## 4. 原有 Skill 工程验收链路全部重跑

| 机器工作流 | 固定 HEAD | 结果 | 证据 |
|---|---|---|---|
| Rust codec+E1a/E1b Skill | `7ef598cfcdec10a574c2724c6a7857bbf4937939` | SUCCESS，原生 3 unit + 6 integration，`E0,E1a,E1b=DONE` | [Run #38758066676](https://github.com/2023violet/ShareReady/actions/runs/38758066676)，[Artifact #11983664227](https://github.com/2023violet/ShareReady/actions/runs/38758066676/artifacts/11983664227)，ZIP sha256 `b646ae1cbb9ef38c1cbcf61d83e7adf0fb554af8a4632849831b8520ace47b8d`；内部状态 tar sha256 `35ac253a6922e186adc6ed79aebcba6990c132582da5196a7b12a61378a30da6` |
| WASM/Chrome+E1c/E1d Skill | `862c13f509236d895f260fdb81d20db06887752a` | SUCCESS，真实四份 PNG 下载、负向 WASM API、Clipboard 条件测试，`E0–E1d=DONE / E1e=TODO` | [Run #38758308909](https://github.com/2023violet/ShareReady/actions/runs/38758308909)，[Artifact #11983939582](https://github.com/2023violet/ShareReady/actions/runs/38758308909/artifacts/11983939582)，ZIP sha256 `a945cf372e2b2c7761458f28ead3a60386e09c7be4de02a9002c9bdcd9c81d11`；内部状态 tar sha256 `c16ba8cba0380acdc421b1e96a53967190d9ee721e2d40fe9dc309f182ba1989` |

第二条 workflow 明确下载**本轮第一条** core 新状态 tar，并校验 SHA256 后才恢复，避免使用修复前的旧 E1 证据链。

## 5. 独立审查者需要重新验证的事项

本项为 **E1e R03 定向整改**。独立审查者应至少：

1. 按上述固定代码版本，从 PR #10 复核 `redact` 的空列表、零面积、负向、完全越界、边界相切与 1×1 有效区域是否具有一致且明确的失败封闭行为。
2. 独立查看 RED Run 中旧代码确实可导出原图，以及 GREEN Run 同一负向输入在 Rust/WASM 双端均无返回 PNG。仅看 200 或 workflow PASS 不够。
3. 独立检查输出文件：当 Rust CLI 接收全越界矩形时应返回非零且不会创建输出文件；真实 PNG 解码后应满足源像素/非遮挡像素断言；不能仅靠界面黑框。
4. 检查测试与生产 API 是否使用同一 `redact` 实现；指出被排除的威胁模型和是否需要更严格的“遮挡面积 / 覆盖范围”业务规则。
5. 签署带**审查者身份、准确 Git commit/blob SHA、实际核验步骤、R03 结论（PASS/REWORK/INSUFFICIENT_EVIDENCE）**的独立评论或报告。不能因一个问题修复就自动签署全部 R01–R10 安全项。

**独立审查状态：`REVIEW_REQUESTED_BUT_NOT_ACCEPTED_OR_SIGNED`。** 已通过 GitHub REST 尝试请求 Copilot bot 复核，但 PR review 列表未返回审核意见或确认过的 reviewer；因此不可以声明审核完成。需要仓库维护者在 [PR #10 Reviewers](https://github.com/2023violet/ShareReady/pull/10) 正式请求 GitHub Copilot 或另一名独立人员；即使 Copilot review 完成，仍不等于人类独立安全审计。

## 6. 不应扩大解读的风险边界

- Clipboard 记录含 `write_succeeded_readback_unverified`；未证明实际 paste-back。
- JPEG decoder 之间允许的小数值像素误差不代表字节精确一致。
- 恶意 EXIF/PNG/ICC 及高强度 fuzz、跨浏览器环境和整体产品威胁模型未完成独立审查。
- Stage 01 的真实用户需求 A1 仍 OPEN，S05a BLOCKED_BY_RECRUITMENT。
- E1e 第六步始终 TODO。不得新建假 `E1_SPIKE_INDEPENDENT_REVIEW.md` 冒充审核、不允许自动合并、发布或改 main。

**阶段判断：`FIX_VERIFIED_BY_CI / REVIEW_PENDING / NOT_RELEASE_APPROVED`。**
