# P03-I 独立审查

结论：**ACCEPT_WITH_LIMITS / 本批局部证据可接受**。本轮发现的生成物读回、PNG 完整结构和测量身份问题均已闭合，未发现仍需阻断本批局部归档的缺陷。性能预算仍为候选，G04、完整产品、安全认证和发布验收均未因此通过。

审查者为独立子代理，未实施产品修改。审查期间未编译、启动应用/浏览器、操作真实剪贴板或重跑 benchmark；实际执行的是离线身份、统计和像素检查。原始审查、脚本、真实命令输出及详细数据保留于 `target/p03-i-evidence/`。

## 精确身份与修正

预期 HEAD `94f7f419aee91faf6f846349d40a897d0d72d7d0`、分支 main；完整 75 项 source_map 与当前源码一致。普通 EXE 的 55 项身份是其中一致的子集。根/Native 锁字节保持，V1 源码、冻结样本与金样合同的只读 git diff 检查无差异。

| 产物 | 身份 |
|---|---|
| 完整性能源码 build ID | `72345b581e5aab2313712fa2e14775c6d8e5b0f64bcd9fb4cd1983a55d2f4a2b` |
| 普通 EXE SHA256 | `1c534cda14c181dc38afff80e50498527818bfc9cf6bb160e558c489780da2cb` |
| 性能窗口 EXE SHA256 | `d3f9c23e656bd92c69e978647f8f37905be6a2ad5b378da90a090ab163f328c1` |
| 串行最终报告 SHA256 | `6f05ef2779a166e3c8dd3c503752162c6be8a49353dc3aa935631a54ca0350b4` |
| 窗口报告 SHA256 | `36b8b807a02a20b2cb183196b25a8cb645ad3c3f12c9cde933ee1fecca23a157` |
| 浏览器报告 SHA256 | `206bab5e408f6bdc373d792b271b612d735e573b375c58586f8bc83054793ad4` |

已复核源图 25MiB/24MP 与生成 PNG 边界分离：当前 Review/Copy 票据持有的 PNG 才进入 RGBA8、非交错、128MiB、24MP、精确尺寸读回；普通文件/剪贴板导入没有大图 fallback。完整 chunk 检查以 checked 长度、逐块 CRC、首 IHDR、连续 IDAT、唯一 IEND 和精确 EOF 拒绝 metadata/APNG/尾随载荷。解码器分配限制仍明确为尽力约束，不能证明来源、遮挡有效性或 OS 内存上限。

初审指出“末尾 IEND 匹配不能证明完整结构”，实施者随后实际复现 duplicate IEND/junk+IEND 被默认 decoder 接受，并增加严格检查与反例；该发现已闭合。遗漏的冻结 fixture 身份、继承 CARGO_TARGET_DIR 与固定 EXE 路径不一致的问题也已修正。测量中的 callback/frame 混淆、复核图未命中遮挡、日志写盘污染、模拟 cap 影响字体图集等问题已有对应修正和反例。严格像素标准与 NEAREST 未放宽。

## 审查者实际执行的检查

实际命令均 exit 0，使用本地已配置 Python；没有重新执行 Rust/UI/浏览器任务：

```text
python -B target/p03-i-evidence/reviewer_runtime_check.py
python -B target/p03-i-evidence/reviewer_final_check.py
```

- 独立核对报告、命令 stdout/stderr、EXE、原始事件/内存采样、40 张性能图和当前源码身份；重新从原始 JSONL 分组、求和及计算中位数/nearest-rank P95/最大值，与派生摘要逐项一致。
- 实际运行 12 次严格全图 PNG oracle：组件 grid/wide 两张、窗口实际保存九张、大生成物一张；large 显式使用 `normalized_source=True`，默认 25MiB source 拒绝仍实际检查。Copy 的 36,000,000 字节 RGBA 与完整输出真值 hash 一致。
- 另外使用独立的 1:1 source crop＋Pillow 遮挡合成，检查 grid、wide-native、forced512 三张中央复核图及 forced512 100% 图，各 584,226 个可见内域像素全部一致。forced512 实际四 tile，内部 x=512/y=568 接缝也在比较内；原生宽图与 forced512 中央像素 hash 相同。
- 对浏览器已保存的五个真实 WASM 输出及一张 UI 下载图，再实际运行六次全图 oracle；五张输出分别匹配冻结 RGBA 金样。核对当前 web 源、复制 site/WASM、工具归档/EXE、浏览器记录与全部声明日志身份。首轮错误的隐藏按钮断言失败被保留；修正后的测试增加 edit 状态 handler 拒绝下载和再次编辑使 outputBytes=0 的负例，没有修改 V1。
- 逐项核对 [ARTIFACTS.json](ARTIFACTS.json) 的 279 个已列工件大小与 SHA256、六张文档图片与原图、[METRICS.json](METRICS.json) 与原始摘要、[RESULTS.json](RESULTS.json) 的报告引用及完整源码、根/V2 入口与本批 README。当前表述明确区分 32 张 100%/200% 原像素图与八张仅作预览的 Fit 图。

详细结果：`reviewer-runtime-data.json` SHA256 `06610cf489607e6d59a94032ea85eaf38e1c63e6b8f73f866cf0c6e65590fa21`；`reviewer-final-data.json` SHA256 `211631100c50d13712daf4fede4511afb72d2aeb5220c43a3077422d55412f2e`。对应 stdout/stderr、脚本 hash 和逐文件身份均保留，前者记录 12 次 PNG＋四次独立 GPU crop，后者记录六次浏览器输出 oracle 与 279 工件核对。

主 workspace 58、Native 48＋桥接 4、feature Native 53＋桥接 4、方法 Python 8、普通 PNG oracle 19 的成功属于已读取且核对日志 hash 的实施者实际运行结果，不冒称审查者再次执行。release CLI 五金样也已核对当前二进制/日志及冻结 RGBA 身份；其原始 verifier 的临时 PNG 未保留，审查者没有重跑 CLI。浏览器五输出原文件则保留，已按上文实际重验。

## 观察与计量结论

审查者逐张查看普通八图：中文与恢复动作完整，编辑态有手柄、复核态为实心 PNG 遮挡；固定检查器不遮挡工具栏/图片，700×480 浅深主题布局正常，确认模态明确。另亲自查看三种配置中央复核、forced512 原像素图及浏览器 review/recovery 图，未见新的画面阻断。这些是应用 GPU 读回/浏览器截图，不是物理屏幕或人类任务验收，也不证明自动识别全部敏感内容。

| 独立复算指标 | 结果 |
|---|---|
| 五次首图 GPU 读回就绪 | 中位 1348.8644ms，最大 2434.9608ms |
| 240 交互帧 controller＋所有 UI pass CPU，含转换帧 | 中位 1.02955ms，P95 1.8144ms，最大 74.359ms |
| 240 帧间隔，含转换帧 | 中位 8.35715ms，P95 23.2324ms，最大 76.1717ms；8/240 超过 33.3ms 参考值 |
| 二十轮导入 controller | 中位 502.156ms，最大 517.4835ms |
| 二十轮复核 controller，另含初始复核共 21 次 | 中位 1778.871ms，最大 1904.7852ms |
| 二十轮内存 | 2461 样本；抽样工作集最大 759,275,520B，OS 生命周期峰值 759,975,936B |

释放后 20 个样本中有 **11 个已位于下一轮导入 phase 之后**；当前成品仍保留。这些数据不是稳定空闲、清空所有产物或无泄漏证明。应保留全 30 帧分布，另列去掉首个转换帧的 29 帧分析，不能只选较快系列。

本轮首图读回比先前历史轮次的 5.07s 快，但 OS 缓存/后台负载未控制，也不是相同条件的因果试验，不据此宣称优化加速。首图回读不是冷机、首帧呈现或首次可操作时间。同步导入/复核仍约 0.5/1.8s；100ms 内可见忙碌反馈尚未验证。CPU、帧间隔、读回和 GPU timestamp/物理响应互不替代，后两项未测。

## 必须保留的限制

当前设备为 Windows10 19045、约 16GB、GTX1650；不代表 Win11/8GB 参考档。采样工作集、OS 峰值、private commit 和独立 GPU 显存分别记录，2GiB/120 秒是试验保护线。两张低熵 24MP PNG 与一张高熵 9MP JPEG 不覆盖全部合法输入。Chromium153.0.8010.12 补验记录 `chromiumSandbox:true`、新上下文和空权限列表；审查者核对了配置/产物，没有做 OS 级沙盒证明，也没有重新启动浏览器。它不能替代 Windows WebView2、真实 OS 剪贴板/跨应用回贴、混合 DPI、Narrator/IME 或人类核心任务。

候选预算、G04、完整离线包、依赖许可/全面安全及发布门槛保持开放，P03-G 仍为已停止的能力分支。下一批处理已测同步阻塞时仍需新的任务/版本身份、取消与过期结果边界和 genuine gates。本报告不授权提交、推送、发布或修改权限。后续只有文档导致工作区 fingerprint 改变时，可以诚实执行 **IDENTITY_RECONCILIATION**，重校验现有实际产物的 source_map/全文件 hash；不得把它描述为再次运行 benchmark/test。
