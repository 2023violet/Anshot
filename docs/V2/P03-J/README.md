# P03-J：后台复核与当前产物权限

状态：**本地机器证据及独立审查完成，受限接受**。本批不代表完整产品、G04预算、物理双屏、Narrator或发布验收通过。P03-I是历史基线；J的当前79项输入和真实产物见 [RESULTS.json](RESULTS.json)。

复核PNG的Rust编码、生成物解码、ColorImage及Fit缩略图准备移入一个后台线程。界面可继续绘图和编辑；取消会撤销旧结果的接收/交付资格，CPU仍会结束当前工作，期间保留唯一任务槽。来源导入、纹理上传/可见分块和系统对话框仍可能同步阻塞。

复核结果只由持有同一来源Arc、document/revision/operation的封闭工作产生。主线程只接受当前结果，并等待实际PNG在最后一个绘图pass留下可见绘图记录、累计frame推进后开放交付。进入等待状态的整个frame跨所有pass拒绝Copy/Save。该绘图记录证明egui提交，不证明物理屏已呈现或人类已检查。

## 固定版本与实际执行

- main / HEAD：`94f7f419aee91faf6f846349d40a897d0d72d7d0`，没有建立分支或提交。
- 79输入build：`1a19169d7317ac939123aad8171a8eb70adf6b123d678ad76046b20faf97fb93`。
- release异步诊断EXE：`1d78847e91d2b4ed736f18d3f254528637ab46ffcf083dd78330b98cd07dee01`。调用真实NativeApp/worker，使用固定grid24MP和noise9MP；诊断输入被隔离，未使用系统剪贴板。
- 普通featureless debug EXE：`582423bba03951f3ebd86779d822f46e6482a1a61e04f9137d44c3c16f1c5d90`；八种真实GPU状态、19项PNG检查器及全图oracle通过。release诊断和普通debug分别记录，不能相互代替构建身份。
- 根66项；普通Native64＋bridge4；feature Native70＋bridge4，无忽略。Empty→send/exit、最后pass无图、同frame多pass交付、modal、取消/异常/迟到结果、cap变化和真实重复Close事件反例均具名执行。
- 8组性能窗口、40张GPU图，每组30不同交互frame；包括20轮重导入/复核。大9MP JPEG生成超过25MiB的PNG仍能复核/受控Copy；来源25MiB与生成物128MiB边界保持。五冻结goldens逐像素通过。
- 完整原始命令、源映射、图片和内存采样分别见RESULTS中的五个report；测试工具保留历史`p03-i-evidence`/`p03-c-evidence`目录命名，不把本次结果写回I历史文档。

## 实测时序

单位ms。Worker CPU从真实开始到CPU显示准备完成；忙碌frame的整个callback起点至App返回均严格位于该区间。P95为nearest-rank。回调包含诊断controller和实际App.ui，排除后续行收集、截图请求及integration/GPU；详细App.ui/控制动作、全回调离群值、进程RSS见 [METRICS.json](METRICS.json)。

| 样本 / 任务 | 触发返回 | Worker CPU | 实际忙碌frame | callback中位/P95/最大 |
|---|---:|---:|---:|---:|
| grid / first | 0.0629 | 1860.32 | 216 | 0.226 / 0.354 / 49.545 |
| grid / cancelled | 0.0615 | 1807.16 | 218 | 0.228 / 0.334 / 0.659 |
| grid / retry | 0.0706 | 1846.67 | 221 | 0.229 / 0.340 / 0.398 |
| noise / first | 0.0585 | 1202.22 | 132 | 0.274 / 0.380 / 61.202 |
| noise / cancelled | 0.0596 | 1149.06 | 139 | 0.243 / 0.353 / 0.708 |
| noise / retry | 0.0882 | 1173.69 | 140 | 0.242 / 0.360 / 0.405 |

每个样本都有busy/current/retry GPU图、两张全图PNG与对应全RGBA受控Copy。重试前实际移动遮挡框，重试输出像素与首轮不同。取消及其返回均发生在真实CPU工作期间，旧key在编辑前已失效，旧结果丢弃且没有晋权。取消尾部按观测到的slot/旧结果收尾计时，不冒称CPU被中断或线程结束的精确瞬间。

当前机器为Win10 19045 / i5-10200H / 约15.83GiB，实际窗口使用GTX1650 DX12；adapter报告16384、实际egui/device cap8192分别保留。GPU清单含虚拟显示设备，不据此推断本地/远程操作或物理屏幕模式。内存为自有子进程RSS/OS峰值/commit，不含独立GPU显存；无泄漏或8GB设备承诺。

这些数字证明异步分工及所测窗口可继续处理回调，不证明图片处理CPU更快；未做冷启动或人类首交互延迟测量。所有回调的最大值和合成取证I/O均保留，不只报告均值或P95。

## 反例、失败与独立审查

- 原同步测试helper的15失败保留，改为等待真实任务/PNG绘图，保留原断言。
- [Native初审](NATIVE_BOUNDARY_INDEPENDENT_REVIEW.md)发现Empty后send/exit最终drain与确认Close失效缺口；两项修前决定性RED真实失败，修后通过。
- [R1复审](NATIVE_BOUNDARY_R1_RECHECK.md)发现实际重复Close事件会重置关闭守卫；新增真实事件RED后采用terminal latch。[R2独立复审](NATIVE_BOUNDARY_R2_RECHECK.md)接受静态/headless边界范围，没有代替GPU验收。
- 三次失败pilot分别为诊断嵌套Context锁、截图/成品同名被create-new拒绝、Fit残留强制偏移使PNG不可见而被AwaitingDraw阻断；全部原始记录保留，未放宽绘图/像素/30帧/120秒/2GiB标准。
- 一次修复测试因运行期间代码改动被genuine流程拒绝采用，未算作通过。genuine累计两失败时均重新规划并更新真实A/B，代理确认按已批准本地自治政策记录，不冒称人类逐token审阅。
- [最终独立审查](INDEPENDENT_FINAL_REVIEW.md)接受本批机器范围，实跑8个全图oracle、5份完整RGBA、6个独立GPU内域比较、原始时序/内存和身份核验；A1领域与Native headless结论仍按其各自范围引用。

## 后续边界

物理双屏/混合缩放、Narrator、真实剪贴板跨应用、Win11/8GB、人类任务、安全审查、包与发布仍开放。遵循 [Windows验收](../WINDOWS_ACCEPTANCE.md)，不以GPU readback替代。Core/WASM与V1源未改，I的浏览器证据保留为历史，不声称J重新运行浏览器。

下一步先关闭本批独立证据审查与不删除归档，再推进同步导入/交付及可访问性缺口；完整产品目标保持进行。


## 证据收口

工作线程阶段的ms是当时捕获的单调时间点；frame/pass是记录行发出时观察的上下文，CPU阶段行可能在显示准备后才发出，不能据其frame字段定位编码完成帧。真实忙碌帧使用独立callback时间区间，绘图/晋权使用实际draw/promoted行验证。

文档变化会改变genuine流程的全工作树指纹；最后的receipt命令只重核未变的79项编译/测试输入、已实际执行的报告/原始文件/EXE和完整文档清单，标为IDENTITY_RECONCILIATION，不冒称再跑benchmark、Rust测试、浏览器或像素oracle。源码任何漂移必须真实重跑。

receipt已用2个正例与12个拒绝负例核验，包括空清单、漏单文件、漏整个root及其全部files、重复root/条目、额外条目、错误hash和畸形文档行；原始夹具保留。raw roots从五个真实report父目录、历史pilot及保留CLI快照集合推导，不以manifest自己的声明作为唯一范围。
