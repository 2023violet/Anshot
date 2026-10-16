# ShareReady E1e｜R04/R08/R09 证据补齐与独立审查交接（2026-10-16）

> **技术验证已补齐指定用例；正式 R01–R10 独立安全审查仍 PENDING。**
> 本文件由参与工程整改的一方编写，**不是第二审查者的签署文件**。不授权真实私密截图、合并、部署或发布。

## 精确冻结的代码与机器证据

- E1 PR #10：`spike/e1-rust-wasm-feasibility` HEAD `13f91fe238b832dd4d629aa99c417b2bb73d84ba`（包含 R04/R09 证据脚本和新增工作流）。
- E2 PR #12：`feature/e2-workbench-integration` HEAD `a1e41f6e264ccfd7138cfca87baeea9c1085032e`（包含实际 CSP、Chromium 负向用例以及 CSP-safe Playwright 等待机制）。
- 既有 E1 核心与真实 WASM：[Run #38828996290](https://github.com/2023violet/ShareReady/actions/runs/38828996290) 和 [Run #38828996278](https://github.com/2023violet/ShareReady/actions/runs/38828996278)，**PASS 于核心源码 Blob 版本；新增 E1 文件不改变 Rust 核心源码**。
- R04/R09 最终：[Run #38835904722](https://github.com/2023violet/ShareReady/actions/runs/38835904722) **SUCCESS**，[Artifact #12019206291](https://github.com/2023violet/ShareReady/actions/runs/38835904722/artifacts/12019206291)（`r04-native-resource-report.json`、`r09-locked-dependency-inventory.json`、`r09-osv-lookup.json`）。
- R08、E2 全量 Rust/WASM/Chrome：[Run #38835725374](https://github.com/2023violet/ShareReady/actions/runs/38835725374) **SUCCESS**，[Artifact #12018594548](https://github.com/2023violet/ShareReady/actions/runs/38835725374/artifacts/12018594548)。
- E2 源码匹配 Chrome 快速回归：[Run #38835724958](https://github.com/2023violet/ShareReady/actions/runs/38835724958) **SUCCESS**。
- 产品环境只允许 SHA256 固定的 T01_CHAT/T02_ORDER/T04_EDGES/T05_JPEG_EXIF 四份合成图片。不能用任何真实私人图片代替。

## 新增证据：实测结果及其严格边界

| 要求 | 观察到的机器结果 | 仍缺的独立证明 |
| --- | --- | --- |
| R04 PNG 36MP 超限 | 65-byte 合法结构头部被 Rust CLI 拒绝，退出 2，未生成 PNG，约 1ms | 更复杂敌意 PNG/JPEG 和解压炸弹 corpus |
| R04 输入超过 25 MiB | 26,214,401 bytes 输入在格式解码前失败封闭，退出 2、无输出，约 12ms | 分配上限证明；所有输入格式与宿主内存边界 |
| R04 截断 IDAT | 45-byte 截断 PNG 返回解码失败，退出 2、无输出 | fuzz、属性测试与极端分配跟踪 |
| R04 有效高压缩率样本 | 2,200×2,200 (4.84 MP) 合成 PNG 被真正重新编码、输出 99,680 bytes。Linux 子进程 `ru_maxrss` 在四个样本的累计观测最大值约 65,000 KiB | **没有测量 24MP 最大值、WASM 峰值/OOM 或受限设备；不能推出产品内存上限** |
| R08 严格 CSP | 实际 Chromium 捕捉 `script-src-elem` 对 inline 的阻断，及 `connect-src` 对 `https://example.invalid/blocked-egress-test` POST 的阻断，0 远端实际请求；Rust/WASM 合成图成品复核成功 | **当前是静态页面 meta CSP，不是生产 HTTP 响应头验证**；其他浏览器/部署链需复核 |
| R08 原有功能兼容 | E2 full + quick CI 均 PASS，四份真实 PNG、三布局 DPR2、键盘、Clipboard 实际 Ctrl+V 匹配、复制 race 负向测试不回退 | 实机/跨浏览器和供应链联网审计 |
| R09 Cargo 锁图 | 精确锁文件 SHA256 `8f77b39445a87f9a57d832e51342fe2eef002ed45a87dd2bb8df28f7df43ab5c`；32 包，许可证元数据字段缺失 0 | 许可证条款/NOTICE/再分发义务真实法律审查 |
| R09 OSV | Live `api.osv.dev/v1/querybatch`：31 个公开 crates.io 包，查询时点匹配漏洞 **0** | 这是 OSV 快照，**不包括 wasm-bindgen-cli 安装用的额外 build graph，也不等于 RustSec/漏洞全覆盖或永久安全** |

### 当前 CSP 限制

`default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' blob:; img-src 'self' blob:; style-src 'unsafe-inline'; font-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; worker-src 'none'`

- `wasm-unsafe-eval` 仅用于明确的 Rust WASM 编译；未添加 `unsafe-eval`。
- Playwright 的 `wait_for_function` 依赖页面内 JS eval，会在严格 CSP 下失败；已改为可信 DevTools 直接函数求值轮询。不得因此弱化真正的 CSP。
- `style-src 'unsafe-inline'` 仍允许样式内联，浏览器 CSP 不能替代未实现的生产服务器安全响应头。

## R01–R10 验收表（交给**非实施者**填写最终 Verdict）

| 编号 | 本轮工程状态 | 允许独立审核者考虑签署所需的剩余证据 |
| --- | --- | --- |
| R01 | EXIF 1–8 / 畸形 TIFF 机械测试通过 | 更多畸形段及 EXIF offset 敌意 corpus |
| R02 | 源像素、逐像素遮挡和真实 PNG 工件通过 | 独立再解码核对极端矩形 |
| R03 | 零有效区域 fail-closed Chrome/WASM PASS | 独立验证无原图回退 |
| R04 | **测试样本的安全拒绝与 native RSS 表征 PASS** | 最大图片/WASM 内存、OOM 与对抗性 fuzz |
| R05 | PNG 严格 chunk/EXIF 测试通过 | 未知恶意 metadata corpus |
| R06 | stale copy / revision 和替换图片负向测试通过 | 宿主异步剪贴板残留风险复核 |
| R07 | Chrome 真正 Clipboard Ctrl+V RGBA 一致 | Edge/Firefox/Safari 或明确的受支持浏览器范围裁决 |
| R08 | **Chrome CSP inline 与外发 POST 拦截 PASS** | 发布版 HTTP CSP 与部署后 Network 审核 |
| R09 | **Cargo inventory + live OSV 31 crates PASS** | wasm-bindgen-cli build chain；许可/SPDX/NOTICE；RustSec/其他漏洞源 |
| R10 | **正式独立 GitHub PR reviews = 0** | 与实施者不同的真实审查者身份和 SHA 锁定、逐项签署 |

## 禁止“无签审也通过”

1. 审查者必须与实现者不同；不得让本报告作者在 GitHub review 中自签 GO。
2. 每项 R01–R10 填实际 PASS / FAIL / INSUFFICIENT_EVIDENCE，加上具体命令、重现工件和检测平台。
3. 只有独立审查者明确写出 GO / NEEDS_REWORK / SWITCH_HOST / NO_GO 等 Verdict 后，才考虑原 Skill E1e 第六步的真实确认；CI PASS 不能替代。
4. Stage 01 A1 真人需求研究仍 OPEN；两个 Draft PR 均不合并、不部署、不发布；禁止私密截图。
