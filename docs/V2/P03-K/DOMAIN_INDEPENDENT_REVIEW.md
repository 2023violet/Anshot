# P03-K Task1 独立领域审查

Verdict: DOMAIN_ACCEPT_WITH_LIMITS

未发现阻止进入 Native 接入阶段的 Task1 领域阻断。接受的是封闭 SourceCandidate 的验证/消费、失败原子性与同步兼容边界；不是 K 最终产品、后台导入、UI性能或系统剪贴板验收。Native 尚未接入，本报告不将其计为完成。

本轮只读产品代码、规格和既有证据；亲自运行获准的 Editor 领域测试、哈希及只读Git保护检查，仅创建新target证据，没有修改源码/锁/fixture/已有文档、提交或发布。main / HEAD 实际核对为 `94f7f419aee91faf6f846349d40a897d0d72d7d0`。

## 实际独立命令与身份

命令：`C:/Users/16429/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/cargo.exe test --locked -p shareready-editor`。显式设置该目录的 RUSTC、RUSTDOC，并把同目录置于PATH首项；实际 rustc 为 `1.98.1 (48a229cea 2026-09-01)`。

退出0，墙钟14.173秒；**30库测试＋2 workflow＋2 compile-fail文档测试全部通过**，0 failed/ignored/filtered。四个新增行为测试均具名执行：

- source_candidate_keeps_exact_validated_bytes_and_dimensions
- source_candidate_replacement_revokes_old_review_and_history
- source_candidate_overflow_preserves_entire_existing_document
- source_candidate_invalid_input_preserves_existing_review

两项compile-fail分别验证外部不能构造私有字段、不能Clone候选；它们实际由rustdoc执行，不是仅源码目视判断。原26+2领域回归保持。完整命令/环境、输出、具名测试、79项输入前后SHA保存在 `reviewer-domain-20261018T140956640419Z.json`，SHA256 `a4a7c1738f7028cbe0a4bc9aae647390813e10c20a80869a50a143348ffa6f1d`；同前缀 stdout/stderr 为完整原始输出，其SHA也在JSON。

精确审阅/运行的 `crates/shareready-editor/src/lib.rs` SHA：`c903c26c9c2e61968043a6fce79b12e2719cdca58201b0323c5232d9c685616e`。测试前后79项输入完全一致；对照 `baseline-j-source-map.json` 79项，只有此文件从 J 的 `3e79cce40918bbdd1cc45b82fca83f2cd03cd3fb16d085ec5895c4772bfb49ac` 变化，其余78项不变。

## 代码审查

1. SourceCandidate 的源Arc和宽高字段私有，未实现Clone；唯一正常构造路径 decode 消费Vec并调用现有 Core decode_source 获取尺寸，再持有不可变原始源。bytes/dimensions只读，不暴露可变引用或Arc构造权；候选不是 reviewed PNG 或某个Editor的丢弃授权。没有新增unsafe/依赖，也未改变Core源25MiB/24MP或其他格式/生成物限制。
2. from_candidate 仅以已验证尺寸创建Editor并移动源Arc；replace_candidate 仅调用既有replace_document并移动Arc。接受阶段没有调用decode_source/redact，没有第二次解码或编码。这个结论来自直接代码路径审查；本轮没有加入解码调用计数探针或测量UI延迟。
3. replace_document先校验尺寸、checked_add document_id，再修改尺寸/源/stamp并清mask、selection、undo/redo、active gesture和所有旧输出/pending/delivery。ID增长失败在任何修改前返回；之后replace_candidate只做不会返回错误的Arc移动，不存在中途错误后的部分替换。overflow测试实际保留旧source Arc身份、stamp/尺寸、mask/selection/history、计数、output及待Copy字节。
4. 成功替换增加document身份、重置revision并使旧ReviewKey/结果、Copy票据和gesture失效；同字节重新导入也不能复活旧权利。from_source/replace_source保留签名并复用decode→candidate消费；无效源在触碰现有Editor之前拒绝，已有复核/Copy仍可按原凭证结束。候选消费失败会丢弃该候选对象，但不改变旧文档，符合当前接口。

Native后续仍必须用自己的请求key、互斥、当前编辑确认及cap规则决定是否可消费候选。候选可被明确交给另一个Editor，本身没有任务新旧/用户确认权；不能仅因“已被Core验证”就允许迟到替换。该点在规格中已明确，不要求领域候选伪装成旧Editor能力。

## RED与保护检查

已读取并逐SHA核对实现者的API编译RED（exit101）和拒绝stub下行为RED（26 passed / 4 failed）；分别保留Missing API与四项真实行为断言失败，不将编译RED称为运行漏洞复现。两轮原始源码前后SHA一致。RED由实现者执行，我未改回旧代码重跑。

根/Native锁、总方案Markdown/PDF四项固定SHA实际吻合；只读 `git diff --exit-code HEAD -- e1-spike/src web-app desktop-windows docs/research/fixtures tests/golden/v1_3_baseline.json` 退出0。这是指定tracked路径与79项输入范围的保护证据，不是未列出目录/未跟踪文件的全仓不变证明。补充记录 `reviewer-domain-supporting-checks.json` SHA `5a43b198cd215f0f11ef53d5f5e938ecdf6d787a7173b2200e5d49d2254f717a`。

规格SHA `cb06d2869aa476036ee34195b5358755e8c2231e473d6cd6b461dca62dcd31bc`，实施计划SHA `3bf27e2dc65c7348e2ed84283e0ae162b4c66870d0d2955679304bd43bf36dd8`。genuine Step1已记录done；实现者另跑根workspace的结果不冒称本次独立执行。

## J源码快照及限制

另实际核验 `target/p03-j-evidence/source-archive-20261018T141035447772Z/files` 的79份源字节逐项匹配 J 最终 source_map，包含原 J Editor SHA；archive report SHA为 `7796e1790406807ec6aa318e258b63797cd642a1d20cabecf0a71d1d97d8d176`。我的只读记录 `reviewer-j-source-archive-check.json` SHA `932eeff5ce718336786d12e047da7d50b127470f18741422989138c7ff9fe437`。这是保存历史源码身份的补充，不是独立重建EXE，也没有扩展或重写J最终22/512清单。

本轮没有运行Native、GUI/GPU、后台source worker、文件/剪贴板真实导入、benchmark、像素oracle、浏览器、系统交付或人类任务。任务取消/迟到错误、互斥、modal/cap、Close终态、真实忙碌帧与全部K回归仍属后续步骤。完整产品继续进行，不能用本领域先决接受关闭这些门槛。
