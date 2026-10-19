# P03-J 独立最终审查

Verdict: ACCEPT_WITH_LIMITS

接受本批**有界后台复核在当前本机、固定合成样本和明确机器证据范围内的实现**。未发现仍阻止该局部结论的代码或运行证据缺口；不接受据此宣布完整产品、G04预算、安全认证、物理/人类验收或发布完成。最终文档收口和包含本文件的新清单仍需独立重校验。

审查者为独立子代理，不是实现者。只读产品和既有证据，运行获准的领域/headless测试、离线像素oracle、哈希和原始数据复算；没有修改产品/锁/fixture/工作流，没有启动真实GUI、性能子进程或系统剪贴板，没有外传、提交或发布。main / HEAD 实际核对为 `94f7f419aee91faf6f846349d40a897d0d72d7d0`。

## 精确版本

最终79输入 build：`1a19169d7317ac939123aad8171a8eb70adf6b123d678ad76046b20faf97fb93`。完整当前源码映射、四份EXE、实际报告和日志身份已独立核查，6项I既有输入修改、4项新增与RESULTS所列delta一致；Core/V1/锁等其余映射不变。

主证据 `target/p03-j-evidence/window-20261018T133030637394Z/report.json` SHA256：`9ba1be1e7064b8ee204972a3ff5ffccb5317d9320b057e37d3552bebcdc294c1`；对应 release async EXE SHA：`1d78847e91d2b4ed736f18d3f254528637ab46ffcf083dd78330b98cd07dee01`。ordinary、fix、large、window四份报告与主报告的源映射关系、各自EXE/日志另在独立JSON中绑定；普通八态EXE是featureless **debug**，不能冒称release产品验收或与release诊断EXE混用身份。

本次文档复核所读 RESULTS SHA 为 `8353677680c2236d845465e8f5b203f87e64ec7dae70b56fd0ff0db22438eec9`，ARTIFACTS SHA 为 `83a217a780573165d7a21cb8ec50c8f28b26ed0c44ff113ec5a05271a3113d9a`，处于独立审查PENDING草稿状态。finalizer随后写入本结论/SHA会改变这些文档身份；上述值不是预先认证未来版本。

## 独立实际做过的验证

- A1亲自执行 `cargo test --locked -p shareready-editor`，26库+2workflow通过；随后按各冻结候选运行Native focused。最终R2执行async_10项和close_4项（含2重叠，共12个不同测试），均通过；R1最终drain等4项review_task测试所在源码与当前字节相同。完整命令、RUSTC/RUSTDOC、原始stdout/stderr和前后身份在原领域/Native/R1/R2报告中，不把跨版本记录合并成当前全回归执行。
- 实际运行 `python -B target/p03-j-evidence/reviewer_final_runtime.py`，退出0。8个完整PNG oracle通过：async grid/noise的first/retry四图、large一图、grid/wide-native/forced512各一图。5份受控Copy RGBA与PNG解码的全部字节相等，first/retry像素确实不同；JPEG明确使用Rust-normalized源且显式normalized_source=True，默认25MiB源边界未放宽，不能称独立JPEG解码器认证。
- 另写Pillow裁片/遮挡期望，对async四张100%完成图与wide/native、forced512中央图共6个可见内域逐RGB像素比较通过，覆盖原像素、遮挡边缘/1px线及分块路径；不是仅抽点或使用Rust输出作期望。重核255个文件哈希、原始事件流、11份内存采样，复算8性能case各30不同交互frame。
- 实际运行新文档复核脚本 `reviewer_docs_controls_recheck.py`，退出0：510项草稿raw清单的范围、大小与SHA全部一致，10份文档图片与原图相同，METRICS派生数值逐项与raw一致；保留夹具上的2正例、12拒绝负例均独立重新调用当前validator函数通过。没有执行receipt接受主流程或向它注入伪接受。

实现者实际全回归日志中的root66、Native64+bridge4、feature70+bridge4及五个critical具名测试已独立读取核查，**未由我另行全量重跑**。19检查器、40张性能GPU图及五冻结goldens的运行记录/字节身份已核查；五goldens的RGBA SHA与冻结合同逐项相同，CLI快照与实际执行前后生产者映射相同。我没有独立重跑CLI五金样或浏览器，原始临时CLI解码结果也不冒称已重新产生。

详细独立证据：

- `target/p03-j-evidence/reviewer-final-runtime-data.json`，SHA `d9cbc06ac96fa6d5b2d1434f401e2c4d4d1d55b59f6a9b2c3aaf2d2a96d6cba4`；执行脚本SHA `2df6206f92c46f29ee74bd3f083a839f494838a562f6110ab47f481347db5c28`。
- `target/p03-j-evidence/reviewer-docs-controls-recheck-data.json`，SHA `a291baf1150364c9f4f40d9e97379e0167f51c3ce90f1af36e55e03f5570c8eb`；执行脚本SHA `858c5df698be655accfa793cef782fc22623b3e259d77a91da553b50f3920b91`。

## 发现、修复与当前边界

独立初审发现poll在首次Empty后send/exit会丢队列结果、确认Close未撤销key；R1又发现真实重复Close事件重置关闭守卫。均有保留的行为RED和修后具名GREEN，R2终态latch及此前最终drain已在获准headless范围复验。私有key/result/envelope继续绑定同一源Arc与完整ticket，旧成功/错误清理不改新状态；取消保留活动worker唯一槽，确认退出后才join；当前PNG最后绘图pass与累计frame推进、入口等待frame跨pass拒绝交付都保持，未绕过原PNG/票据边界。

三次失败pilot的诊断嵌套锁、截图/成品同名、Fit残留偏移均诚实保留。不可见PNG被AwaitingDraw拒绝晋权，是守卫生效证据，不能把旧失败pilot当最终通过。修复过程中代码变动导致genuine拒收的一次green亦保留。代理确认genuine gates来自已批准连续本地授权，不冒称人类逐token审阅。

最终receipt初版遗漏整个root仍可自洽的覆盖缺口已收紧：五组实际reports、四历史pilot与快照推导精确根集合，显式证据集合独立于manifest，空/整组遗漏控制均拒绝。其后独立发现CLI原执行键的反斜杠与快照正斜杠直接比较会误拒绝；helper现以Windows Path相等且SHA相同的唯一原始执行键填入快照，未改产品、79输入或checker。这一证据修正已实际独立核对，原失败审计保留。checker SHA `aafae2a13f99770f151146a4a7248e54610487c55398398c195178f8523a9420`；它只作identity reconciliation，不冒称重跑测试、计量或像素oracle。

## 观察与计量限制

真实CPU工作期间的完整callback帧独立复算：grid为216/218/221，noise为132/139/140；取消及方法返回均落在相应工作区间内，旧结果无晋权。取消至观察到stale收尾分别约1801.01ms/1141.53ms，不能称线程立即停止。首轮忙碌callback最大49.55ms/61.20ms；所有已记录callback最大354.24ms/287.08ms也保留于METRICS，低P95不能掩盖合成I/O等离群值。方法触发返回约0.06–0.09ms不是总处理时长、物理反馈或因果加速。

亲看最终grid busy、noise busy/first/retry、wide/native与forced512中央图；grid两张完成图及普通8态与已实际查看图逐字节相同，复用这些观察。中文与忙碌/取消按钮可见，完成图是实心遮挡且无编辑手柄，重试遮挡实际移动，右栏/工具栏没有此前浮窗遮挡。busy为Fit预览，没有原像素精确断言；没有取消尾部专门GPU图，不能声称我看过取消后的视觉提示。合成trace模式完成图仍禁用OS交付按钮，不能由截图单独证明普通模式交付。

以上均为当前Win10/GTX1650/约15.83GiB本机的程序化证据。egui绘图提交不等于GPU fence、OS物理呈现或人类复核。普通导入、可见tile准备/上传、对话框和交付仍可能同步阻塞；source25MiB/24MP与生成PNG128MiB保持，不证明所有合法24MP高熵输入性能。采样RSS、OSpeak、commit与GPU显存不同，20轮也不证明无泄漏/稳态；8GB/Win11、物理双屏/混合缩放、Narrator、真实跨应用clipboard、人类任务、安全/包/发布及完整产品仍开放。Core/WASM未变，J没有重新运行浏览器。

当前草稿checksum已按其当时20项payload（加manifest自身共21文件）核对；**未认证尚未生成、包含本文件的最终SHA256SUMS**。finalizer还须同步README/V2入口与计划状态、绑定本报告和新增审查证据，再进行独立最终checksum/清单重校验。以上受限接受不替代这些收口事实，产品继续IN_PROGRESS。
