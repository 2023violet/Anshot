# ShareReady E1｜独立安全与架构审查任务书（等待其他审查者）

> **2026-10-15 · E1e REVIEW REQUEST / NOT AN INDEPENDENT REVIEW / DO NOT MARK PASS**
>
> 本材料由代码/实验实施方汇总，不能签署为“独立审查通过”。所需的真正第二审查者尚未确认。原门禁中 E1e 的 `verify_cmd` 仅检查正式报告文件存在性，**不能作为人类独立审查的替代**；在独立审查实际完成前不得创建冒充报告，也不得在原 Skill 中将第六步标记 DONE。

## 固定范围与代码审查起点

- 独立 E1 工程分支：`spike/e1-rust-wasm-feasibility`，PR [#10](https://github.com/2023violet/ShareReady/pull/10)，目标仅为 Stage 01 研究分支，**不是 main**。
- 原 Skill：`deciding-and-executing v4.1.0`、固定 SHA `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`。
- 真正 Gate A/B 已通过，E1 任务 1–5 的通过状态只能以各自独立 CI 和恢复后的 `.decision` 证据为准。
- 核心代码：`e1-spike/src/lib.rs`、`e1-spike/src/main.rs`、`e1-spike/Cargo.lock`；
  浏览器：`e1-spike/web/index.html`；
  验证：`e1-spike/tests/codec_security.rs`、`verify_browser_export.py`、`verify_clipboard_and_memory.py`。
- 数据：仅 [冻结合成夹具](../research/fixtures/manifest.json)，禁止真实私人截图；E1 实验网页实际还有 SHA256 夹具白名单，禁止输入其他图片。

## 必做对抗性审查

| ID | 需要独立检查/验证的漏洞或不确定性 | 最低证据 |
|---|---|---|
| R01 | JPEG APP1/EXIF/TIFF 方向解析的越界、offset 溢出与畸形 tag；方向 2–8 的所有变换是否正确 | 属性测试、单元测试、变形 JPEG 及具体失败样本 |
| R02 | 带反向坐标、负坐标、图像四角、1px 极窄框、多框重叠的真值区域，是否真的在输出 buffer 不透明替换 | 从独立生成的输出 PNG 二进制重解码并逐像素比较 |
| R03 | 零个有效遮挡框/全越界框可能导出未遮挡图片；是否应在试验代码或产品版禁止 | 显式风险裁决和失败封闭规则，不以 UI 文案代替 |
| R04 | JPEG、PNG 恶意长度/畸形或图像解压炸弹，25MiB 压缩/24MP 像素候选限制是否在任何分配前有效 | 无用户数据的压力测试、峰值内存和 OOM 降级证据 |
| R05 | 新 PNG 是否可能包含源文件 EXIF/GPS、私有 text、ICC 或原图层字节；测试是否遗漏未知 Chunk | 二进制 Chunk 审计、恶意元数据 test corpus，不只检查 `eXIf` 四字节 |
| R06 | 真实编辑/复制/下载是否严格由同一 revision 的最终 output bytes 驱动；无状态竞态或回退成原图 | 真实浏览器故障与异步竞态测试 |
| R07 | Clipboard 真正写入、读取和权限拒绝在 Chrome/Edge/Firefox/Safari 的差异；未回读是否被误认成完整通过 | 实际权限日志与粘贴回读，不允许模拟成功 |
| R08 | 网络/CSP/依赖是否带出源图片/成品、静态包是否依赖外部 CDN；区分 `blob:` 与远端 HTTPS | Chrome DevTools/Playwright 网络记录、受限测试 |
| R09 | Cargo.lock 32 包及构建环境 wasm-bindgen-cli 的许可证与安全公告；能否构建 reproducible release | SBOM/许可证/安全审计、来源 SHA |
| R10 | 同一 Agent 写代码和自查的偏差、自动化 headless-only 与小样本所缺的证明 | **由独立于实现者的审查者**提交带证据和 commit SHA 的意见 |

## 结论状态模板（未来独立审核者填写，不能预填 PASS）

- 检查代码准确 commit SHA：
- 审查者与职责（须独立于 E1 实施者）：
- 浏览器/平台环境：
- 真正执行过的命令及 CI / Artifact URL：
- 每项 R01–R10：PASS / FAIL / INSUFFICIENT_EVIDENCE + 具体证据：
- 阻断项、需要整改的差异和修复范围：
- Verdict：GO / NEEDS_REWORK / SWITCH_HOST / NO_GO / INSUFFICIENT_EVIDENCE
- 明确说明：技术 GO 不是 Stage 01 市场 A1 已验证，不批准 main merge 或产品 Release。

**当前状态：独立审查 PENDING。** 仅确认现有同一 Agent 的机器实验和可追踪资料，不宣称所有安全证明已通过。
