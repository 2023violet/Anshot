# G01 至 G10 验收差距表

基线：94f7f419aee91faf6f846349d40a897d0d72d7d0。所有源码判断为静态核对；CI 结论引用固定 EXE 的历史运行，人工结果本轮均 NOT_RUN。源码缺口不写成实机 FAIL，存在实现也不写成人工 PASS。

## 总表

| 实验 | 合同完整范围 | 当前实现及机器证据 | 剩余缺口 | 所需修改或取证 |
|---|---|---|---|---|
| G01 | T04 四角 Fit/100%/200% 指针与全图像素 | Rust Viewport、绘制与滚动画布存在；T04 EXE 自检只测一框与版本 | 缺 Fit/精确缩放入口、物理比例语义和可核对指针记录 | M01/M02/M03；TC-G01-01～03 |
| G02 | 八手柄、移动、1px、键盘、一次撤销 | Editor 八向调整、手势和历史已有测试；POC 有八向 1px 按钮 | 手柄缺可见反馈；完整键盘创建/移动及 Esc 取消未接入；实机精度未知 | M03；TC-G02-01～03 |
| G03 | 100/125/150/200% 和混合 DPI 多屏 | 浮点图框映射及缩放几何有 Rust 测试 | 实际 pixels_per_point、搬屏、指针轨迹及全图像素无证据 | M01/M02；TC-G03-01～02 |
| G04 | 真 24MP、GPU、内存和帧时 | 2048px 显示纹理，全分辨率导出；CI Dx12/Cpu 窗口通过 | 缺原像素复核、确定性 24MP 样本、帧时测量；同步工作与重复解码需量化 | M01/M02，M09 条件触发；TC-G04-01～03 |
| G05 | 原生选择/取消、Explorer 拖放及无损恢复 | rfd 选择和 dropped_files 处理存在；解码失败不换文档 | 换图直接 replace_source，无丢弃确认；先无界 fs::read 再由 Core 查 25MiB | M05；TC-G05-01～04 |
| G06 | 图片粘贴导入与复制到 Paint/第二程序回贴 | arboard set_image 有实现；无 get_image/图片粘贴入口 | 粘贴缺失，DIB/alpha/跨应用像素及失败恢复未测 | M04/M06；TC-G06-01～04 |
| G07 | 另存、取消、只读、覆盖/源路径冲突、重试 | create_new 拒绝已有文件，write_all/sync_all；取消能报失败 | 新建策略与原合同覆盖语义有差异；直接写目标后的半成品及重试边界未闭合 | D03/M06；TC-G07-01～04 |
| G08 | 中文 IME、全键盘、Narrator | AccessKit feature 开启；英文 UI；撤销/重做/删除与可聚焦按钮存在 | 未设置中文字体，核心键盘路径不完整；焦点/快捷键输入保护及语义未测 | M03/M07；TC-G08-01～03 |
| G09 | 修改/替图/撤销与过期 Review/Copy/Save | 文档/版本/操作令牌、拒绝旧回调已有 Rust 测试 | GUI 未有延迟交付测试缝；新绘制与现有手势起点的失效规则需逐路核对 | M01/M09；TC-G09-01～03 |
| G10 | 包、离线、网络、依赖/许可与锁身份 | EXE/锁/编译/自检身份已核实；两个 wgpu 探针历史成功 | 真实包断网使用、进程网络、许可无证据；POC cargo test 0 项不可冒充领域回归 | M08；TC-G10-01～04 |

M 项是候选修改，尚未执行；D 项是需评审的产品策略，详见最小清单及 Gate 候选。测试用例全文见协议与 `TEST_CASES.json`，不能因标了编号就当已有结果。

## 源码锚点与判断依据

以下链接全部固定到待测 SHA，不随 main 漂移。行号在本轮实际读取并核对。

### G01 G03 与 G04

[app.rs L26](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/poc/native-egui/src/app.rs#L26) 的 display_texture 将最长边缩至 2048，start_review 复用同一方法。L248 的 render_canvas 使用来源宽高乘 zoom 作为逻辑尺寸，L321 是连续滑块。没有 DPI 除数、Fit 计算或可视原像素纹理路径。此处确认实现方式，未测其实际视觉表现。

坐标和像素需分两种证据：指针映射由实际图框与事件推算期望区域；全图输出由冻结输入加记录的真实 mask 独立合成。不能只用 GUI 自报的矩形检查自己的输出，就宣称指针映射准确。

### G02 与 G08

[app.rs L172](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/poc/native-egui/src/app.rs#L172) 有八向命中，L197 起有手势处理，L279 的键盘逻辑只直接读取 Z/Y/Delete；L248 起绘制填充区域，未画八向手柄提示。侧栏八向按钮可用性要实测，不据此声称完全无键盘能力。Editor 的 [begin_gesture L179](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/crates/shareready-editor/src/lib.rs#L179) 和 cancel_gesture L203 已存在，优先接入，不复制新状态机。

未发现 set_fonts 或中文字体资源初始化，也未发现键盘创建遮挡、移动、Escape 取消、关闭前保护及文本输入焦点防抢处理。这里是对当前源码的缺口核对，不是完整无障碍审计。

### G05 G06 与 G07

[app.rs L66](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/poc/native-egui/src/app.rs#L66) 直接 fs::read，然后准备 Editor 和纹理，再 replace_source；没有统一丢弃确认。L117 保存取消会 finish(false)，已有路径由 create_new 拒绝，但写入/同步失败后没有处理本次目标的残留。L140 只调用 Clipboard::set_image，没有图片读取入口。L359 清空直接调用 Editor，未接确认 UI。

输入限制仍在 Core 中存在；缺口是平台读取先分配再查限，不能错误描述成 Core 没有上限。保存覆盖拒绝是当前明确策略，不是已经测出的覆盖漏洞；是否沿用必须经 D03 确认。

### G09

[Editor L260](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/crates/shareready-editor/src/lib.rs#L260) 的 begin、L284 bytes_for、L293 finish 已校验身份。L374/L412 等用例检查已有手势和旧回调；POC 的生成、文件对话框、保存与复制在 UI 路径同步调用。测试应区分“领域可确定性注入延迟”和“GUI 当时是否能发出第二动作”，不可记录不存在的人工竞态。

新绘制在 GUI 内先有 DragMode::Draw，已有框手势则进入 Editor::begin_gesture。需要专门验证这两个起点的失效语义；本轮未执行反例，不将其定性为已利用的隐私缺陷。

### G10 与测试入口

[native workflow](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/.github/workflows/v2-03-native-poc.yml) 仅对独立 POC 运行 cargo test，实际日志为 0 项；33 项领域测试来自主产品 CI。渲染步骤允许 continue-on-error，三个窗口任一通过就可使探针脚本返回 0。因此最终 Windows 主路径验收必须读 wgpu/DX12 的具体状态及适配器。

[golden verifier L40](https://github.com/2023violet/ShareReady/blob/94f7f419aee91faf6f846349d40a897d0d72d7d0/tests/golden/verify_v1_baseline.py#L40) 对无 .exe 后缀的路径先 is_file；L58 的 JPEG preview 同样如此。它是现有 Linux CI 入口，不能未经适配就把原命令写成 Windows 端已可运行。保留 Linux CI 真值；后续为原生导出加文件读取式检查工具，或单独评审平台适配，不在 P03 A 顺手修脚本。

## 评审时不能被省略的缺口

G06 的粘贴导入不能从新版验收表中消失；G07 的拒绝覆盖不能默认当作旧保存体验等价；G04 的软件窗口不能抵消物理 GPU/24MP 缺测；G08 的按钮可聚焦不能替代键盘完成编辑任务；G09 的 Rust 测试不能直接升级成 GUI 人工通过。
