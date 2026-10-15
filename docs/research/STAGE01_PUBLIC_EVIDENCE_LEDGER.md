# Stage 01｜公开证据台账与竞品能力差异

> **SR-RES-003 · 更新日：2026-10-14 · 状态：DOCUMENTARY REVIEW ONLY**  
> 本记录只描述当日检索到的公开页面；没有打开竞品产品实际完成任务、计时或做安全测试。任何未找到的功能均记 `NOT_VERIFIED`，不可写成竞品“没有”。

## 判定术语与来源独立性

- `DOCUMENTED`：所引官网、产品文档或仓库确实**声称/记载**此能力；不代表我们已经证明其实现可靠。
- `MEASURED`：在固定设备/版本及可重复测试样本上实际操作和记录；**本轮没有任何 MEASURED**。
- `HYPOTHESIS`：对用户需求或差异化机会的推断。
- `NOT_VERIFIED`：功能、表现或合法性尚未核查。
- 网站作者的营销描述与项目代码、自主测试、独立用户反馈属于不同等级；不得把“官网说”当“独立验证”。

## A. 直接竞争者的官方记录

| ID | 竞品/来源（2026-10-14 检索） | 官网明确列出的相关能力（DOCUMENTED） | 同任务需测什么（NOT_VERIFIED） |
|---|---|---|---|
| C01 | [Redacted features](https://redacted.app/features) | 本地处理，无账户，剪贴板粘贴/复制，拖放，多个可调整遮挡，黑框/模糊/像素化，PNG/JPEG 导出，清除元数据，首次加载后离线使用，免费 | 输入至输出耗时、最终实际输出像素及元数据、安全性、中文场景 |
| C02 | [Redacted AI](https://redacted.app/ai-redaction-tool) | 浏览器本地 OCR、规则匹配、NER 名称检测并可调整遮罩，强调仍需要人工复核 | 本地模型准确率、中文名字识别、漏检和误报；不要直接继承其营销结论 |
| C03 | [Redacted screenshot guide](https://redacted.app/guides/how-to-redact-a-screenshot) | 明示共享前检查姓名、头像、网页地址、通知、订单等区域；明确推荐敏感文本使用实心遮挡 | 与 ShareReady 引导复核原型的比较；不能再说“竞品没有检查清单” |
| C04 | [Redaction Tools](https://redactiontools.com/redact-screenshot) | 粘贴/拖放 → 识别并检查遮罩 → 复制/下载；本地分析；英语 OCR 模型、可选 AI | 复杂非英语截图、处理时间、漏检率、人工交互负担 |
| C05 | [ShareX image editor](https://getsharex.com/docs/image-editor) | Windows 截屏后编辑、形状/涂抹及图像处理；可连接保存/复制到工作流 | 新手对完整任务的耗时；与系统截图路径比较；具体版本须本机核对 |
| C06 | [Redacted iOS](https://redacted.app/ios) | 官方描述 iOS 分享菜单、iPad/macOS 原生处理，本地 AI；免费功能与付费功能分层 | 分发限制、免费功能边界和真实使用速度 |
| C07 | [Redacted Windows](https://redacted.app/windows) | 官网显示 Windows 原生版尚在开发/提前登记，宣称本地处理 | **不能**把待发布页面当成 Windows App 已运行证据 |
| C08 | [已存在的 ShareReady](https://shareready.app/) | 同名且同样以分享前图像处理为定位 | 正式商标/域名法律清查，避免与现有品牌混淆 |

## B. 功能是否已经同质化？

| 原拟议卖点 | 公共证据 | 评审结论 |
|---|---|---|
| 完全免费 Web 遮挡 | C01 | **不能作为差异化** |
| 浏览器本地处理、无账号 | C01、C04 | **不能作为差异化** |
| 粘贴截图 → 框选 → 复制 | C01、C04 | **不能作为差异化** |
| 最终检查是否遗漏信息 | C03、C04 | **已有检查清单/复核流程**，不能宣称独有 |
| 本地自动识别私人信息 | C02、C04 | 现有工具已声称实现；加入 OCR 还会增加开发范围 |
| 源图像像素替换与元数据处理 | C01；需实测 | 是产品安全**下限**，不是竞争优势 |
| 对特定日常截图任务显著更少失误/步骤 | 未见独立可比测试数据 | **唯一值得优先验证的结果假设**，未证实 |
| Rust 实现与完全开源核心 | 产品架构选型 | 有利于代码透明度与可验证性，但普通用户不一定因此迁移 |

### 初步结论

**COMPETITIVE RED FLAG**：重复实现同类功能很容易做出产品，却难以证明必要性。Stage 01 不允许将“更简洁的 UI”作为不需验证的口号；必须在明确的截图类型、操作方式和用户分组中证明结果差异。

## C. 浏览器实现文档（约束，不是产品实测）

- [MDN Clipboard.write](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/write) 描述在安全上下文中写入图片的 API；是否成功仍受用户手势和具体浏览器约束。
- [MDN ClipboardItem](https://developer.mozilla.org/en-US/docs/Web/API/ClipboardItem) 描述图像 MIME 支持与兼容差异。
- [Leptos CSR deployment](https://book.leptos.dev/deployment/csr.html) 描述 Trunk 生成静态 dist 后可静态部署；这**不等于**首屏性能、浏览器输入与隐私安全已经达标。
- [Leptos CSR getting started](https://book.leptos.dev/getting_started/index.html) 明确首屏 JS/WASM 载入和 SEO 的权衡。

## D. 研究空白（必须通过执行取证）

1. 真实用户在过去 30 天中**自己**遇到多少次分享前隐私处理任务，是否在传统聊天/收据/论坛技术求助中反复发生。
2. 竞品在同一套合成截图上表现如何；是否能在实际结果里观察到遗漏或过度遮挡。
3. 安全与效率是否冲突；分享前额外复核环节到底帮忙还是妨碍。
4. 用户为何需要从当前截图工具切换到 Web 页面；若无切换动机，应停。
5. Rust/WASM 的加载、图片内存、复制/下载是否与预期行为一致。

**研究结论状态：DESKTOP_EVIDENCE_COLLECTED / USER_NEED_AND_COMPARATIVE_ADVANTAGE_NOT_VERIFIED。**
