# ShareReady NEXT-01 / NEXT-02｜安全复核与验收证据状态（实施者以外签署前的预审）

**日期**：2026-10-15  
**性质**：基于 GitHub 实际源码的同助手静态/证据复核；**不是独立于 E1/E2 实施者的安全审查**。  
**结论**：NEXT-01 `NOT_VERIFIED_ON_LATEST_HEAD`；NEXT-02 `NEEDS_REWORK / INDEPENDENT_SIGNOFF_PENDING`。不得创建冒充签署的 `E1_SPIKE_INDEPENDENT_REVIEW.md`，不得将 Skill E1e 标记 DONE。

## 1. 追踪基线

- Draft [PR #10](https://github.com/2023violet/ShareReady/pull/10)，分支 `spike/e1-rust-wasm-feasibility`，本轮源码最后写入提交 `01e2e60b6f60a53521230a138f754780794a665f`。
- Draft [PR #12](https://github.com/2023violet/ShareReady/pull/12)，分支 `feature/e2-workbench-integration`，CI 重构最后写入提交 `acdee6ccee12bf856200bedcc461547b971832cb`（最终验收需重新查询实时 HEAD）。
- GitHub PR #10 与 #12 的正式 review submission 数量均为 **0**；[Issue #11](https://github.com/2023violet/ShareReady/issues/11) OPEN。
- 两个 PR 的状态均为 DRAFT、未合并；当前 E2 白名单只接受 T01/T02/T04/T05 冻结合成图片。绝对不使用真人或私人截图。
- 历史 E1 RED/GREEN [#38757327352](https://github.com/2023violet/ShareReady/actions/runs/38757327352) / [#38757489210](https://github.com/2023violet/ShareReady/actions/runs/38757489210)、E2.2 全源码 [#38765702714](https://github.com/2023violet/ShareReady/actions/runs/38765702714) **只能证明当时的提交，不可继承为以上 HEAD 的结果**。

## 2. NEXT-01 当前状态

| 要求 | 进展 | 本轮验收 |
| --- | --- | --- |
| E1 Rust 单元/负向测试、`Cargo.lock --locked` 一致性 | 测试、固定锁文件及 CI 实现已提交 | **未拿到最新 HEAD 运行记录** |
| E1 Rust→WASM 编译、真实 Chromium 四夹具导出及逐像素解码 | 既有浏览器验证脚本，新增复制版本竞态与回读不匹配负向用例 | **未拿到最新 HEAD 运行记录** |
| E2.3 键盘创建、移动/缩放、焦点恢复及无障碍状态 | 已实现 JS、HTML、新 Chromium 测试 | **未拿到最新 HEAD 运行记录** |
| 原生浏览器缩放 | 新测试通过 CDP 设置 Chromium `Emulation.setPageScaleFactor`，并追加真实鼠标绘制原图像素坐标校验 | **新测试未跑；不是浏览器菜单 Zoom 兼容结论** |
| PNG 真正 Clipboard Ctrl+V 回读 | 新脚本使用 Chromium 权限、真正键盘粘贴事件、解码 RGBA hash；失败时阻断 | **新测试未跑；不是跨浏览器证书** |
| 源码 SHA / 构建工件 / 可重复产物核验 | E2 full workflow 从当前 commit 重编 Rust/WASM；快速 CI 已移除旧 E1 WASM artifact 的静默重用，改为从当前源码编译 | **源码已修改；尚无新 CI 工件验证** |

**当前环境限制**：GitHub 连接能读取源码、指定运行编号的 CI，但不能枚举上述最新 push 的工作流运行；本地有 Chromium，却无 `cargo/rustc` 且浏览器无法访问 localhost 测试服务。单独 CDP visualViewport 缩放冒烟能够运行，但**不等价**于 Rust/WASM 端到端验收。不得把缺失的真实日志写成 PASS。

## 3. 本轮发现和针对性修复

- **A01 / P0 / FIXED_CODE_NOT_RUN**：E1 PNG Clipboard 实际读回与源字节不一致时，历史分支可能仍提示“写入成功但回读未验证”。已修成显式不匹配告警，并追加故意制造不同 readback 字节的 Chromium 负向测试。注意 OS Clipboard 异步写入不可撤销。
- **A02 / P1 / FIXED_CODE_NOT_RUN**：E1 `e1Debug.copiedStatus` 在源图片变化或重新生成输出时保留上次值。已在 `invalidate()` 与新 generation 时重置 copy 状态及回读错误，防止状态被误用为现行复制证据。
- **A03 / P1 / FIXED_TEST_NOT_RUN**：E2.3 CDP visual-viewport 缩放测试原仅通过键盘建框/导出，不能证明指针坐标准确。现已补入真实 Playwright pointer draw 及源像素坐标容差断言。
- **A04 / P1 / FIXED_WORKFLOW_NOT_RUN**：E2 快速工作流此前比对过时 Rust Blob `ab4871a...` 并重用旧 WASM artifact；与当前源码 `36119d23...` 不符。已改为 source-matched wasm32 编译，避免假阳性或必然失败的旧哈希门禁。
- **A05 / P1 / FIXED_WORKFLOW_NOT_RUN**：E2 全量工作流触发器未包含共享 E1 浏览器、codec 及 Clipboard 测试文件。已扩展 trigger，防止 E1 负向验证更新被漏跑。

以上代码修复未经过最新 HEAD 远端真实 CI。该章节只描述实现事实，不宣称测试通过。

## 4. E1e R01–R10 对抗性预审矩阵

| 审核项 | 预审发现 | 下一步必要证据 / 当前判定 |
| --- | --- | --- |
| **R01** JPEG EXIF 方向/解析 | `jpeg_orientation` 使用带界限检查的 TIFF 读取，但测试仅覆盖 orientation 6 的少量像素，2–8 / 恶意 APP1 输入没有充分证明。 | 所有 orientation 2–8 真值、错序 tag/offset/溢出属性与 fuzz 样本。**INSUFFICIENT** |
| **R02** 几何/实心覆盖 | 核心 `clipped` 对反向坐标、1px、边界与正向合成图有测试；历史真实浏览器导出曾验证像素。 | 最新 HEAD 的独立 PNG 像素 readback + 边界用例。**HISTORIC PASS / CURRENT UNVERIFIED** |
| **R03** 零有效面积 | 核心 `redact` 先后拒绝空矩形和解码后全无有效矩形；历史 RED/GREEN CI 有证据。 | 新 HEAD `cargo test` 与真实 WASM API 负向回归。**FIXED / CURRENT UNVERIFIED** |
| **R04** 恶意图片/OOM | `MAX_INPUT_BYTES=25MiB`、`MAX_PIXELS=24MP`、PNG 36MP CRC 正确头部负向用例已增加；但没有峰值 RSS/WASM memory、JPEG decode 极端压力、畸形语料库。输出需多次分配解码/RGBA/PNG。 | 内存上限、资源耗尽失败封闭、fuzz/property、确定性退出。**INSUFFICIENT / P0 SECURITY BLOCK** |
| **R05** 元数据 | `DynamicImage::write_to` 使用新像素缓冲 PNG 编码，T05 EXIF/GPS 负向校验已存在；但未知 ancillary chunk、iCCP、文本段、尾部附带字节尚缺全量二进制枚举证据。 | 独立 PNG chunk parser + 对抗性元数据 corpus。**PARTIAL** |
| **R06** revision/导出 | 输入变化让输出失效、导出核对版本；新增异步 `write` 后复核及 stale 状态。更换输入时 OS 可能已收到旧剪贴板 payload，前端无法回滚。 | 当前 HEAD 真实异步竞态/错误分支测试，禁止旧状态误导。**FIXED CODE / UNVERIFIED** |
| **R07** Clipboard | 已有权限拒绝与下载回退的历史 Chrome 证据；新增真实 Ctrl+V + RGBA hash（E2）和回读不匹配失败分支（E1），当前未执行；Firefox/Safari/Edge 未测。 | 真实新 CI + 跨浏览器权限/回读矩阵。**INSUFFICIENT** |
| **R08** 网络/CSP | 工作台文件 JS 依赖相对路径，本地服务及历史测试观察 localhost/blob；但缺正式 CSP、响应头、发布后第三方依赖与跨源阻断审计。 | CSP 策略、构建产物请求审计、输出源图不外发动态验证。**INSUFFICIENT** |
| **R09** 软件供应链 | `Cargo.toml` 精确锁定 image=0.25.8 与 wasm-bindgen=0.2.105，`Cargo.lock` 已提交；核心 CI 不再 generate-lockfile。但不存在充分的依赖许可证、公告、SBOM、构建工件来源全面核验。 | 独立 SBOM/license/advisory 与可复现构建证据。**INSUFFICIENT** |
| **R10** 独立性 | 目前 GitHub 正式 PR review 为 0；本报告作者参与本轮修复，所以不能作为独立正式安全审查。 | 未参与实施的另一位审查者提供身份、SHA、逐项证据和明确 verdict。**BLOCKED** |

## 5. 不可越过的 STOP 条件

1. **未找到精确 HEAD 的完整 CI 成功日志+工件**：NEXT-01 维持 `NOT_VERIFIED`，不能进入生产门禁。
2. **R04/R01/R05/R07/R08/R09 不足和 R10 0 正式审核**：NEXT-02 维持 `INDEPENDENT_REVIEW_PENDING`；原 Skill 第六步 `E1e` 不得标为 DONE。
3. 所有 E1/E2 试验继续**仅 SHA256 白名单四份合成图片**，不得开放私人图片。两条 PR 继续 Draft，不合并到研究分支/main、不部署、不发布、不打 tag。
4. Stage 01 人类用户研究 A1 仍未验证，技术验收不得自动代替产品价值判决。

## 6. 下一行动（具体顺序）

1. 在 GitHub Actions 找到精确当前 SHA 的 E1 native、E1 WASM、E2 源码重编全量工作流；逐 Job 检查 conclusion、checkout SHA、命令日志及工件 SHA。若失败，保留 RED 和具体原因，修复后重新跑，禁止复用历史绿色结果。
2. 严格执行真实 Chrome PNG/Clipboard/CDP 缩放、下载二进制哈希与像素验证，其他浏览器单独记录实际能力或限制；必要时追加数据生命周期/峰值 RSS。
3. 对 R01/R04/R05/R08/R09 生成敌意样本、PNG chunk 映射、CSP/网络与 SBOM；记录精确 Rust 代码来源和测试机证据。
4. **邀请真实独立审查者**复核 PR #10 的当时最新 HEAD 并提供 R01–R10 逐项 verdict；不能由本报告作者代签。阻断修复后再次复核。
5. 只有 NEXT-01 实际 PASS 且 NEXT-02 真实签署、经过独立产品授权，才能讨论下一阶段权限；不能自动合并/发布。

**最终预审 verdict：NEEDS_REWORK / INDEPENDENT_SIGNOFF_PENDING。**
