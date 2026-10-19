# P03-K：有界后台导入

状态：已实现并完成本地机器取证，独立审查受限接受；完整产品仍进行。目标来自已批准总方案的LOADING、失败保留编辑和UI阻塞缺口。用户已批准连续本地开发；本批代理按真实流程记录审查，不另请求逐token确认，不扩大外部/破坏操作权限。

## 意图与成功结果

用户打开、拖入或粘贴大图时，已有画布继续处理事件，并能取消候选导入。读取/解码/剪贴板DIB规范化和CPU显示准备在后台完成；有效候选到达且必要的放弃修改确认完成后才替换文档。失败、取消、过期或关闭后的结果不覆盖当前图片、历史、当前成品或状态。CPU取消是撤销接收资格，唯一槽保留到线程实际结束。

当前源处理有Display和Editor两次Rust解码；本批允许两次解码都在worker，优先消除UI CPU工作，不借此改Core格式/像素合同。OS文件选择对话框、系统剪贴板有界字节读取、GPU上传和可见分块仍可能同步。不得称全流程无阻塞或满足尚未冻结的G04预算。

## 方案与选择

1. 选择单个有界来源任务，封闭Editor候选+同源PreparedDisplay。复用J单槽/最终drain/线程结束后收尾的模式；UI只上传、确认和原子换图。无需新运行时或依赖。
2. 只把文件读取移出UI：主要解码及显示准备仍会阻塞，不能达成本批目标。
3. 通用任务池/所有交付同时异步：扩大并发内存、接口和审查面，当前不采用。

## 领域候选边界

在shareready-editor增加不可Clone、字段私有的SourceCandidate，持有Core验证后的不可变源Arc及宽高。`SourceCandidate::decode(Vec<u8>) -> Result<Self,EditError>`运行完整现有decode_source；只读`bytes()`与`dimensions()`供同一worker准备显示，不把普通PNG当已复核成品。

`Editor::from_candidate(SourceCandidate)`和`replace_candidate(SourceCandidate)`消费此对象；接受阶段不得再次解码/编码，所有可失败的document_id增长检查发生在修改之前。既有from_source/replace_source保持签名并复用候选逻辑。接受候选必须清除旧mask/history/gesture/复核结果/交付资格，旧ReviewKey失效；失败和IdentityOverflow不部分修改。

候选本身是有效来源，不是特定旧Editor的编辑权限；可向一个Editor显式替换，不能伪造维度或已复核凭证。Native私有任务key另外约束哪个请求仍能发起该替换。Core/WASM/V1接口与像素处理不变。

## Native单任务与旧状态

私有source_task模块持有SourceInput（File路径、已拥有的PNG/JPEG字节、或已拥有的ClipboardImage）、新Arc请求身份、容量1channel、JoinHandle和不可重配的PreparedSource envelope。worker完成有界读取/既有格式检查、DIB到PNG、SourceCandidate及PreparedDisplay；只允许这些既有来源路径，不增网络或缓存。

App保留一个来源任务槽，并与J的review_task/awaiting-draw互斥：两类start入口均在读取/解码前检查，不排队；source取消仍占槽直到is_finished。poll必须覆盖首次Empty后send/exit的最终drain，不join运行线程。错误/panic/断连/启动失败可恢复；迟到成功和失败仅在当前请求key有效时作用。普通模式不添加遥测；诊断feature且active时才记录合成事件。

开始导入不清旧Editor或复核结果。忙碌期间可继续编辑旧图；禁用新导入、生成复核和Copy/Save，显示“正在打开图片，可以取消或继续编辑”。取消立即撤销请求key并保留旧状态，诚实提示后台收尾；不声称中断文件I/O或CPU。任务收尾后可重试。

有效结果接收时检查当前显示cap，再上传同一候选的显示数据。若当前仍有编辑/手势，取消未提交手势并显示既有丢弃确认；继续编辑丢弃候选，放弃修改才消费领域候选。结果准备完成前的失败不得取消当前手势或擦除历史。候选到达时重新检查当前编辑，不能只检查开始导入那一刻。

已有modal期间延后结果。确认关闭撤销来源和复核请求，清候选显示资格并保持terminal latch；后续真实Close事件、迟到结果和输入不得复活文档或系统交付。无编辑但来源任务活跃时的实际Close同样使请求失效，保留OS自然关闭行为，不制造多次Close命令。

## 接受标准与机器取证

1. 领域有效候选/无效来源/IdentityOverflow/旧复核与历史失效、同步兼容，先RED再GREEN，保持原26+2领域测试。最危险假设先验证：无第二次UI解码也能安全换图。
2. barrier/channel反例证明来源和复核互斥、send后占槽、取消/迟到错误/Empty最终drain、忙碌编辑后再确认、失败保留旧成品和手势、cap改变、modal和重复Close终态。先RED；真实App.show跨帧，不直接越过完成边界。
3. 当前精确release诊断调用普通App路径，固定grid24MP与noise9MP来源，记录触发返回、worker处理、接收/上传和真实busy callback。至少2个完整callback严格位于真实worker区间，实数记录不加延时；不足保留失败并重规划。busy与候选接受后GPU图绑定请求和源身份；取消后旧文档输出不变，重试有效替换并能生成/全图核验新PNG。方法计时不是物理反馈。
4. 根、普通/feature Native+bridge、19PNG检查器、J真实异步复核边界、八普通GPU状态、既有大PNG与五冻结goldens回归；依实际源码映射决定是否需WASM重建，未跑项目不记通过。
5. 独立审查精确代码/原始产物/图/量测，genuine high Gate A/B/verify/review及不删除归档，失败记录保留。双屏/DPI/Narrator、真实系统clipboard跨应用、Win11/8GB、真人任务、安全/包/发布与完整产品仍开放。

## 约束与止损

main唯一开发分支，预期HEAD94f7f419aee91faf6f846349d40a897d0d72d7d0。根/Native锁、V1路径、冻结金样、总方案/PDF维持J固定hash；不清理其他dirty工作，不增依赖，不提交/推送/发布/删除/改系统权限。

源25MiB、24MP、DIB既有限额、生成物128MiB与完整Rust PNG/交付身份边界保持。只运行固定合成fixture、自有新工件/子进程，不接触私人图片或真实系统clipboard。工作集2GiB或120秒无进度停止本批自有试验，保留失败；两失败genuine重规划。不人为睡眠凑busy帧，不因失败改像素期望、预算、golden或权限。

## 决策与计划检查

采用architectural路径：涉及领域接受接口和Native后台所有权，需要书面规格与逐步计划。规格自查：目标、旧状态保留、请求身份、错误、互斥、cap、关闭、取证和非目标均有明确规则；无外部动作。本地代理按连续授权审查书面规格后编制计划，实施者root，独立子代理审查。此处DOCUMENTED，不等于实现/性能MEASURED。


## R1：最终接受能力与异步完成断言

首轮普通Native65通过/12失败，旧导入/粘贴/大生成物拒绝检查仍在调用返回时判断完成。offer_source的Result现在只确认任务提交，完整验证在worker完成后；测试应通过真实App.show或精确poll等待，再保留原source/stamp/history/PNG/格式/alpha/丢弃确认断言，不删除或放宽。旧J迟到review测试改用明确已准备候选注入来测试同一旧消息失效；普通入口另由互斥反例证明不得并发。

独立预审发现接收时cap通过后，候选可能停留在丢弃确认期间；PreparedSource须保存原cap，最终replace_prepared在消费之前再次核当前cap。变化时拒绝、保留旧Editor/history/PNG并可重试。自查旧任务poll在key不匹配时不得take掉别的current key，先反例后按owned key清理。实际重复Close后的键盘N反例RED已保留，最小关闭输入守卫通过本轮全测。

错误来源保留：异步粘贴失败仍明确“粘贴失败”，不能用泛化图片错误丢掉原来的恢复提示；已完成候选与CPU忙碌状态分开。首个cap测试因egui未清理TexturesDelta终止的失败及修后4项worker通过均保留。维持原安全、像素、格式、CPU帧数、2GiB/120秒和授权边界。


## R2：CPU取证边界与受控交付文案

真实首轮pilot检查器混淆已复核旧PNG与当前复核视图，已记录失败；15项方法反例修正并独立实跑通过，CPU覆盖/取消/忙碌GPU请求只使用read_complete..cpu_prepared，禁止以文件I/O补帧。reviewed与review_view、document/revision和唯一后续同request回执分别绑定。

后续完整机器命令通过72根、82+4普通、89+4诊断、两来源窗口、J/普通/大生成物/性能/金样，但root实际查看六图发现source诊断output的受控RGBA consumer结束后保留了普通copy_choice的“系统剪贴板已接受”状态。未使用OSclipboard，不能把此图视为最终诚实取证。原始完整执行和84项源码/V1 producer字节已另存，genuine reopen及第二失败触发本次重规划。

只在source诊断受控输出成功后设置合成取证/未使用系统剪贴板的事实文案，与J诊断一致；普通交付行为不改。这是低影响文字修正，不增加镜像文案的单测。重新运行当前精确源码的完整窗口/像素/回归，并实际查看新图和独立审查。原像素、cap、CPU帧数、预算、golden及授权边界一律保持，不用旧全量绿色代替新版本。
