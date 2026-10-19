# P03-J Native 边界 R2 独立复审

结论：**NATIVE_BOUNDARY_ACCEPT_WITH_LIMITS_STATIC_HEADLESS**。R2 已关闭确认 Close 后由重复关闭事件解除交付守卫的剩余阻断；结合 R1 的代码复审和独立测试，本轮未发现阻止进入 Task3 真实窗口取证的剩余 Task2 阻断。此结论只覆盖明确测试的 Native 状态边界，不等于 GPU、实际 UI 响应、人类复核或 P03-J 整批完成。

原始阻断报告与 R1 复审保留不变。本轮未编辑产品源码、docs 既有文件、锁或 fixture；仅只读审查、执行获准 headless focused 单测并创建此新报告/原始证据。没有运行 UI、GPU/性能子进程、真实 clipboard/station、系统文件交付或对外操作。

## R2 定点检查

`handle_close_request` 对 close_confirmed 直接 return，不再重置为 false。检查当前 app 的赋值路径，关闭终态只在初始构造为 false，没有重复关闭事件的重新开放路径。原 R1 的 sealed key 取消、AwaitingDraw 清除、活动 worker 保槽和 start/Copy/Save/迟到 consumer 守卫保持。

新增 `async_confirmed_close_is_terminal_across_repeated_viewport_close_events` 使用 RawInput viewport 的实际 Close 事件，连续三次经过生产 handle_close_request，检查不再 CancelClose 且 terminal latch 仍真；之后继续 headless App 帧及 late Copy/Save/start，consumer 不触发、无新 worker、状态文本不变。旧 `close_cancel_retains_history_and_explicit_discard_emits_one_close` 仍验证取消确认保留 stamp/masks/undo，并验证明确丢弃发出 Close、后续事件不再取消关闭；仅最后的旧一次性放行断言改为 terminal 状态。`repeated_close_cannot_bypass_pending_preview_confirmation` 仍确认未获丢弃确认前重复关闭不能绕过模态。

这些测试模拟 egui 输入及受控 consumer，不触发实际系统窗口或 OS 交付。late Save 使用 None 受控选择，源码守卫在文件处理之前；不能描述为真实文件对话框/保存集成验证。R1 的最终队列 drain、私有结果/display 绑定与跨 frame/pass 晋权实现未在 R2 改动；相同字节上的 R1 独立证据继续有效，不冒称本轮重新执行全部测试。

## 本轮独立实际命令

使用固定 stable MSVC cargo，显式同目录 RUSTC/RUSTDOC：

`cargo test --locked --manifest-path poc/native-egui/Cargo.toml --bin shareready-native-egui-poc FILTER -- --test-threads=1`

| FILTER | 实际结果 | 命令墙钟 |
| --- | --- | --- |
| async_ | 10 passed / 0 failed / 0 ignored，54 filtered | 24.152秒 |
| close_ | 4 passed / 0 failed / 0 ignored，60 filtered | 5.777秒 |

两个命令均退出0；共14次测试执行，其中两项 Close 测试重叠，合计 **12个不同测试**。新增 terminal Close、原 running/AwaitingDraw Close、旧取消确认与重复关闭保护均实际具名执行。前后10项选定源码与两锁 SHA 完全相同，详细命令/环境/具名结果/原始 stdout/stderr 身份在：

`target/p03-j-evidence/reviewer-native-r2-focused-20261018T131952069949Z.json`，SHA256 `f55ad2c600b3feec3772cffd6ce0b92b71a77a859a63ac460ab9fb9b76d85bcc`。

同前缀 `-1/-2.stdout.txt`、`.stderr.txt` 是完整独立原始输出。它们没有替代实现者 full normal workspace/bridge 的日志，也不表示独立全回归执行。

另外读取实现者 R2 RED：新增 terminal 测试 **0 passed / 1 failed**，断言为 `actual repeated Close event must not reopen delivery`；是行为 RED，非缺 API 或编译错误。原始 stdout SHA `6e52e87b4cc8c7ae83784f671fc273c27aa758f81998787accdae87f5f225378`，stderr SHA `b7d105e11553b97a5f86bcf8fffba410bb2ec985893802633926df14d6a44aba`。RED 由实现者执行，我只读取核查；genuine r2-native-green.json 记录 step2 done，不能把它当我的运行。

## 精确身份与后续限制

当前 R2 app SHA256 为 `b297ca5497c2466e067db8ace6a92d9b57ed6082013a2786308e599d8e7db265`。与 R1 独立 JSON 的同10项输入比较，**仅 app.rs 变化**；以下继续相同：

| 文件 | SHA256 |
| --- | --- |
| crates/shareready-editor/src/lib.rs | 3e79cce40918bbdd1cc45b82fca83f2cd03cd3fb16d085ec5895c4772bfb49ac |
| poc/native-egui/src/review_task.rs | b1bf49bac80dfe51ee1d2a40b6388d65e8490d22676e16f9d319a4e52f0b2f7c |
| poc/native-egui/src/display.rs | d3dd3b5760c7c50cbdf64ecb078e54ad37461680c86d8a6bd363cead1a541ef2 |
| poc/native-egui/examples/support/async_driver.rs | 69b939e64fdc64290a7b5bd58d785d065cd91775a01aa2da9330680f602f2d96 |
| tests/native_oracle/run_async_checks.py | 3995e2c7a20e256c0139f9423b52830956e81d734e5b457d5ef8a9c2aafa9971 |

trace/visual/两锁的完整身份见 reviewer JSON。原两报告 SHA 分别保持 `05ce9d16a6f12bec67ee19c000beae8c5e54651adb47f81f1b2c3b74aaf03c7f`、`80547b769aa1c7b316a1374e73546a24afe3ebcb4667f54371c616d7ba195ed1`。

Task3 的新真实窗口、busy 工作时段截图、PNG/受控 Copy 全图、实际取消尾部/资源和完整回归尚待完成。R1 diagnostic/runner 修正仅有静态认可，本轮未重新运行它们。不得把 headless 图元/texture delta 当 GPU/物理屏、人类复核或 OS clipboard 通过；DPI、Narrator、Win11/8GB、预算及完整产品开放门槛保持原状态。
