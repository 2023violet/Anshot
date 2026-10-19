# SR V2 03 P03 A 评审材料

日期：2026年10月17日。范围：待测版本固定、G01～G10 差距核对、真机协议及最小修改清单。状态：DOCUMENTED / BASELINE_IDENTITY_MEASURED / HUMAN_ACCEPTANCE_NOT_RUN。

## 结论与使用顺序

继续以 egui/wgpu 为主要候选，先在当前电脑上观察真实 GPU、DPI、画布和键盘路径，再按最小清单修复必要缺口。本材料不作 GO_EGUI 裁决，不授权切换用户版本、删除 V1、推送或发布。

1. 阅读本文，确认本次固定的是哪份源码与 EXE。
2. 阅读 [差距表](GAP_MATRIX.md)，区分源码缺口与设备证据缺口。
3. 按 [真机测试协议](WINDOWS_TEST_PROTOCOL.md) 评审实验是否能证明目标。
4. 阅读 [最小修改清单](MINIMAL_CHANGES.md)，确认范围和先后顺序。
5. 使用 [Gate A/B 候选材料](GATE_REVIEW_CANDIDATES.md) 进入真实工作流；A、B 各自展示预览并分别确认。

机器可读输入为 [版本清单](BASELINE.json)、[CI 原始渲染矩阵](CI_RENDERER_MATRIX.json) 和 [待执行用例表](TEST_CASES.json)。`SHA256SUMS` 覆盖本目录各交付文件；它证明内容身份，不证明人工实验完成。

## 固定的待测版本

| 对象 | 固定身份 |
|---|---|
| 源码 | main，94f7f419aee91faf6f846349d40a897d0d72d7d0 |
| Windows 原生 CI | [Run 39010891632](https://github.com/2023violet/ShareReady/actions/runs/39010891632)，相同 SHA，SUCCESS |
| 工件 | [Artifact 12092468260](https://github.com/2023violet/ShareReady/actions/runs/39010891632/artifacts/12092468260)，4,902,745 字节 |
| 工件 ZIP SHA256 | 07e47508af118dacf0b09f0e4b38746056a51ca85e5df23edcf3fca2b32d211d |
| EXE SHA256 | 386200cc23ff8015258b655a6911c1993c8d08e8fef3e57ca826e9757fc76b9d |
| EXE 大小 | 11,279,872 字节，未签名 POC，本轮未启动 |
| 工件 Cargo.lock SHA256 | 20ab491a04245bee12b2687f7fa85f5a1bddcc4302212203f02c55235619b51e |
| CI 工具链 | cargo 1.99.0；rustc 1.99.0；不是仅按 POC 的 MSRV 1.88 推定 |
| 工件到期 | 2026-10-31T06:49:27Z；下载副本不是已批准的永久回退档案 |

已实际下载原 ZIP，并对照 API digest、内部 EXE、CI 日志和锁文件。EXE 哈希同时匹配 ZIP 成员、CI 日志及渲染矩阵中的身份。没有构建新程序，没有执行 `--self-test` 或窗口探针。历史 CI 的软件渲染结果仍只能证明该 CI 环境。

`BASELINE.json` 分别记录 Git blob 与当前文件字节哈希，避免把检出换行差异混为源码变化。18 个关键源码、锁文件和合同输入相对该 SHA 无差异；五张合成输入的实际文件哈希也与冻结清单一致。此处只校验输入身份，未重新运行金样本或任何产品测试。

## 工作区与已有文件保护

开工时有三项上一轮文档内容未提交：`docs/V2/README.md` 的总方案链接、总方案 Markdown 和其 PDF。这些不是产品源码修改，本次保留，不清理、不覆盖、不提交。版本清单保存了它们的起始哈希；本轮只会在 V2 索引增加本材料入口，其余原内容保持。

产品待测版本指上述 Git SHA 对应的代码和 EXE。评审材料本身是当前未提交文档，并不属于该提交；不能将旧 CI 写成已经验证本轮文档。

## 已确认的资源与待确认项

用户已确认可使用当前 Windows 电脑和双显示器。只读系统查询观察到 Windows 10 家庭中文版 build 19045、约 15.8 GiB RAM、Intel UHD Graphics 和 NVIDIA GTX 1650，同时列出 GameViewer Virtual Display Adapter。完整显卡驱动版本保存在版本清单中，没有采集主机名、序列号或用户名。

双显示器各自的分辨率与 DPI、显示连接拓扑、物理 GPU 实际选择、是否经远程会话、操作者及第二个粘贴程序尚待现场记录。CIM 能列出 GPU 不等于样机已使用它。当前机器不覆盖 Windows 11 或 8GB 最低资源参考档，这两个范围继续 NOT_VERIFIED。

## 关键发现

- G01/G03：当前仅有 0.1～2.0 的缩放滑块，没有直接 Fit/100%/200% 动作；100% 的逻辑点与物理像素语义尚未固定。
- G04：编辑及成品纹理均最多 2048px；不能据此验证原像素细节，24MP 样本、帧时和内存证据也尚未准备完成。
- G06：有复制图像代码，未发现图片粘贴导入；原合同要求两者，本轮恢复完整范围，不缩减为只测复制。
- G05/G07/G09：换图、清空与关闭的编辑保护缺口；保存直接创建并写目标，失败时半成品处理需验证；当前系统交付同步执行，不能伪造“异步 GUI 已测”。
- G08：现有快捷键只直接处理撤销、重做与删除；缺少完整的键盘创建和移动路径。AccessKit 开关不能证明 Narrator 核心任务通过。
- G10：构建/锁文件已有机器证据，许可、离线和网络仍未验证；渲染步骤 `continue-on-error` 使工作流总成功不能作为最终图形门禁。

## 本轮完成与未完成

P03 A 的文档与只读取证已经形成；具体交付检查结果见 `PACKAGE_VALIDATION.json`。G01～G10 的人工执行结果均为 NOT_RUN，未签署 Gate A、Gate B 或 GUI 选型。`GATE_REVIEW_CANDIDATES.md` 只提供评审输入，没有真实 preview/token，不伪造 `.decision/` 或脚本 PASS。

下一建议是评审 Gate A 候选，再将通过的方向装入真实 Skill，单独产生 Gate A 预览；A 确认后才生成 B0 基线观察计划的 Gate B 预览。B0 仅观察，不包含产品修改，后续修复要以具体 diff、测试和命令修订 Gate B。
