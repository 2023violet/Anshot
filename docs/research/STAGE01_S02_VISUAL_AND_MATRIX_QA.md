# ShareReady｜S02 测试素材视觉与证据质量验收

**SR-QA-002 · 2026-10-14 PDT · SAME-AGENT VISUAL REVIEW + CI STRUCTURAL TEST · NOT INDEPENDENT UX REVIEW**

## 范围及真实性

这一轮审查了 GitHub 独立分支上真正生成的五份合成图像，包含机器读回测试、合成预览图、T04 四角原分辨率裁剪图以及来源可核查的竞品矩阵。审查者是原执行 AI，不冒充外部独立审核者。对竞品的操作计时、编辑输出质量和用户体验**未进行任何实测**。

- 图像生成：[Actions 38679131718](https://github.com/2023violet/ShareReady/actions/runs/38679131718)；生成器 [generate.py](fixtures/generate.py) 和真值 [manifest.json](fixtures/manifest.json)。
- 第一次原图拼版、导出模拟：[Actions 38680414793](https://github.com/2023violet/ShareReady/actions/runs/38680414793)。
- 原分辨率 T04 四角裁剪与再次检查：[Actions 38680571476](https://github.com/2023violet/ShareReady/actions/runs/38680571476)；拼版审查证据 [artifact 11947093992](https://github.com/2023violet/ShareReady/actions/runs/38680571476/artifacts/11947093992)。
- 原文件均在 [fixtures/](fixtures/)；一个 `manifest.json` 记录图像 SHA256、源像素坐标、敏感区域及需要保留的区域。

## 逐图视觉结果

| 任务 | 本轮直接看见的证据 | 视觉结论 | 尚需说明的限制 |
|---|---|---|---|
| T01 聊天 | 四个分离的虚构敏感元素：头像、发言人、演示电话、通知；保留两处对话 | **PASS**：布局、目标差异和保留文本可区分 | 虚构界面简化，不是实际社交平台截图，不可推算现实识别准确率 |
| T02 订单 | 虚构收货人/电话/地址与保留的金额/物品问题相互分离 | **PASS**：可作为“不能遮住业务信息”的任务 | 没有复杂页面小票或真实中文电商布局 |
| T03 技术报错 | 深色终端、虚构账号/路径/token 与保留错误/Trace 区分 | **PASS**：区分敏感数据与所需技术信息 | 示例 token 特意不是真凭证，不能用作高熵密钥检测准确率测试 |
| T04 高 DPI 贴边 | 先在合成缩略图中确认四角构图；再查看原分辨率四角带矩形边界的裁剪，四处测试标签文字清楚且被框住 | **PASS**：按原图像素核对四角对象，缩略图小字属于展示缩放而非素材缺陷 | 缩小到一般聊天窗口时部分字会不清晰；该样本主要用于坐标/DPI 验证 |
| T05 EXIF/GPS JPEG | 几何风景和虚构摄影者/地点两处文字；EXIF 和 GPS 由 Pillow 读回成功 | **PASS FOR METADATA TEST ONLY** | 插画式模拟照片，不足以评估真实照片编辑的视觉任务表现 |

## 机器验证到底证明了什么

`manifest` 与图像像素尺寸一致，sha256 可复核，所有敏感框及需保留框均在图内且未交叉。研究用 Pillow 在复制件上按指定矩形写入实心像素，再重新编码 PNG；代码检查了左上、右下目标位置的替换色，以及 JPEG 模拟 EXIF/GPS 不出现在生成 PNG 信息中。

**必须特别指出**：这是**研究夹具的非产品模拟**，不是完整像素差分证明，更不是 ShareReady Rust 应用已经实现安全遮挡。实现阶段必须再次验证整个遮挡区域全部像素、alpha、剪贴板重贴、异常输入和元数据块。

## 竞品矩阵复核

| 竞品 | 本次核验 | 状态 |
|---|---|---|
| [Redacted](https://redacted.app/features) | 官网明确描述本地遮挡、剪贴板复制、导出、元数据剥离 | `DOCUMENTED`，未运行 |
| [Redaction Tools](https://redactiontools.com/redact-screenshot) | 官网明确描述粘贴、自动识别、检查和复制/下载；聊天匿名工具亦已存在 | `DOCUMENTED`，未运行 |
| [ShareX](https://getsharex.com/docs/image-editor) | 官方编辑器手册明确截图后编辑、隐私处理及复制/保存 | `DOCUMENTED`，未运行 |
| [Screenshot Redactor](https://github.com/cleanroom-ai/screenshot-redactor) | README 列出本地隐私识别、截图遮挡和演示入口；本轮纠正以前“仓库存在但能力未记载”的过度保守记录 | `DOCUMENTED`，未运行 |
| 操作系统内置截图工具 | 具体 OS 与应用版本未知、需要在测试机器安装/确认 | `NOT_VERIFIED`，不算四款已核查竞品之一 |

因此 S02 的“≥4 个带可核查官方来源的替代工具条目”达成；但**没有任何 `MEASURED` 行**，且 `version_at_measurement` 与 `observed_environment` 均保留 `NOT_OBSERVED` / `NOT_MEASURED`，绝不伪造实际使用版本。

## 验收与下一步

| 交付合同 | 结果 |
|---|---|
| 四款具体工具的可追溯文档能力矩阵 | **PASS（文档基线层）** |
| 五个实际 PNG/JPEG 合成输入 | **PASS** |
| 五套敏感/保留像素坐标真值 | **PASS（结构校验）** |
| T04 原分辨率四角边界目视检查 | **PASS（同一 Agent 内部检查）** |
| 虚构 EXIF/GPS 可被独立读取 | **PASS（测试夹具）** |
| 所有图像的真实场景代表性 | **LIMITED** |
| 竞品同任务速度、安全性、导出结果 | **NOT_MEASURED** |
| 独立人工用户可用性评价 | **NOT_VERIFIED** |

**S02 任务级建议：`READY_FOR_SKILL_VERIFY_AND_DONE`**。仅表示研究素材和文档矩阵就绪，不能推导市场需求成立。接下来的 S03 必须以同一套素材对至少两款现有竞品执行真实输入、遮挡、导出与核查，不能把网页官方介绍转换成测试结果。

**守护限制：不修改 Rust 产品源码、不合并 PR、不发布；A1 用户价值假设仍为 OPEN。**
