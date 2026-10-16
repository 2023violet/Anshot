# ShareReady V1.2｜Windows 本地启动器安全边界 · Gate A 决策候选

> 2026-10-16｜**DIRECTION CANDIDATE / GATE A NOT CONFIRMED / NOT AUTHORIZATION TO MODIFY SECURITY CODE**  
> 固定原始基线：PR #14 `feature/v1-1-windows-launcher-edit-ux` @ `c16b0953a4705a36b5c93a1645704ebd7a4cde7e`；从该 SHA 创建独立 `docs/v1-2-localhost-security-gates`，本分支**只放决策和验收合同**。  
> 统一 SOP：`deciding-and-executing v4.1.0` @ `1eb9624ee2037e4a2b253ee8120fa274dad1ef21`。本 Gate 不继承 E1 Gate A/B token；未获得新预览的用户确认。

## 01｜真正的问题和既有事实

V1.1 Windows 启动器会把静态 HTML、JS、Rust/WASM 通过 `TcpListener(IPAddress.Loopback)` 仅在 `127.0.0.1` 临时端口提供给浏览器。真实机器 CI [#38840885015](https://github.com/2023violet/ShareReady/actions/runs/38840885015) 在 `c16b0953a470...` **PASS**：Chrome 产品操作、Windows loopback、HTTP CSP、POST=405、非法文件路径=404、WASM MIME；两个 jobs 为 `v1-rust-wasm-chrome` 与 `windows-localhost-launcher-smoke`。这是特定用例通过，**不是产品可处理真实私人数据的安全证明**。

源码静态核对显示 `web-app/Start-ShareReady.ps1`：
- 已采用固定路由白名单和 loopback 绑定，因此外部网卡不监听。
- 请求行只在 `ReadLine()` 完成之后检查 `Length <= 2048`；**检查前无单行读取上限**。
- 请求头循环仅控制最多 64 行，不控制**每行长度/累计字节数**；意外 EOF (`$headerLine -eq $null`) 可提前结束循环，却仍可能返回 200。
- 没有检查 `Host` 是否唯一且与 `127.0.0.1:<绑定端口>` 精确一致。非授权站点通过 DNS 重绑定等方法触达本地 HTTP 服务属于**有根据的威胁假设**，现有 CI 没有负向 Host 测试；不能把静态源码观察直接写成已成功利用的漏洞。
- PowerShell 服务采用同步串行 `AcceptTcpClient()`，请求超时 4 秒；多个慢速本地连接可能占用响应机会。此次不承诺解决通用 DoS。

GitHub 已有 V1 Draft PR #13 和 V1.1 Draft PR #14，**均未合并**。E1e Issue #11 和第三方独立 Review 仍未解除；Stage 01 S05a 真人招聘与 A1 需求验证仍开放。

## 02｜候选方向（明确取舍）

| 方案 | 优点 | 缺点 / 适用条件 |
| --- | --- | --- |
| A. 保留轻量 PowerShell HTTP 服务，严格校验 Host 和 HTTP 输入边界（**推荐**） | 不增加运行依赖、不影响 Rust/WASM、Windows 10/11 双击使用；可在真实 Windows runner 复现 | 只能封闭已识别的 HTTP 攻击面，不等于通用服务器认证；仍需独立复审 |
| B. 换用完整框架/带访问令牌的本地服务 | 现成的请求解析/访问控制能力可能更强 | 打包与供应链扩大，令牌生命周期/URL 传播需要新威胁分析；此阶段复杂度过高 |
| C. 迁移到签名原生 Tauri/Electron 安装程序 | OS 隔离/签名等长期方向可研究 | 会引入新的构建、签名、更新、安装权限与产品决策；不适合本次窄范围修复 |

- 结论：**改后做**（仅以 A 方向开发独立 *测试先行* 的候选实现，评审前不改变已验收的 V1.1 分支）。
- 最大风险：以 Host 检查为由引入破坏性启动问题，或误认为「loopback + CSP = 隐私安全认证」。
- 缺的关键证据：**A1**，Windows 原生服务是否在正常浏览器 Host 请求下维持功能，并对非 loopback Host、重复/缺失 Host、恶意/截断 Header 严格失败封闭。
- 置信度：高（静态代码路径与已有 Windows CI 可核对）；对 DNS rebinding 可利用性**尚未进行敌意浏览器实证**，不能判定为已证实漏洞。

## 03｜目标和成功标准

目标：在不更换当前单机静态架构的情况下，给 V1.1 的 Windows loopback HTTP 边界增加可复核的最小输入信任约束，并以正负测试和工件证明没有破坏 Rust/WASM 编辑器。

1. **AC01** 普通 Chrome/Edge 对 `http://127.0.0.1:<实际端口>/` 的合法静态资源 GET/HEAD 仍成功；Windows 10/11 可双击启动，不引入额外安装依赖。
2. **AC02** HTTP/1.1 请求必须提供 **唯一且精确匹配** `Host: 127.0.0.1:<boundPort>`；缺失、重复、恶意 Host、其他 IP/域名、畸形格式应拒绝（4xx），不暴露静态资源字节。
3. **AC03** 请求行在输入缓冲层和头部逐行、累计层面具有限制；超过限额、无终止空行、超时应失败封闭或直接关闭连接，不输出有效资源；不能只在 `ReadLine()` 后检查。
4. **AC04** 固定文件路由白名单、仅 loopback、405 对 POST/PUT、`no-store`、`nosniff` 和 **HTTP 响应** CSP 均不退化；应用不提供接收文件的 HTTP 路由。
5. **AC05** 原生 Windows runner 执行一组真实 TCP 负向协议探针，不依赖 JavaScript mock；Linux Rust/WASM + Chrome 正向和合成图片像素测试全部运行并且当前 HEAD 绿灯。
6. **AC06** 代码差异仅针对 Windows 静态启动边界与相关测试/说明；Draft PR、逐项风险评估、机器日志和另一个审查者意见齐备，**不改变 E1e 现有独立安全 GO 门禁**。

## 04｜不做什么

不迁移 Electron/Tauri；不引入在线账号、遥测、远端上传和第三方 CDN；不启用真实个人截图作为验收材料；不合并任何现存 PR、不发布、不签名，不宣称可对真实敏感材料提供安全保证；不自行确认 Gate A/B 或填造 E1e 第二审查结论。

## 05｜关键假设与失败条件

| 假设 | 可证伪试验 | 若不成立 |
| --- | --- | --- |
| A1 Windows 原生 `Host` / HTTP 输入边界可在极小增量中正确实施 | 先写原生 TCP 负向探针，对旧实现得到失败/不足，再测试候选修复 | 回到方案选择；不因为 CI 难写而放宽 AC02/03 |
| A2 现有启动方式不会因严格 Host 破坏合法浏览器访问 | Windows native GET/HEAD、CSP、真实 Chrome 正向复测 | 暂停，修正 Host/URL 约定，不放行 |
| A3 V1.1 在此改动后仍保持浏览器输入/输出与纯本地数据边界 | Linux WASM/Chrome & synthetic pixel regression 与静态网络核对 | 停止发布讨论，回归最近通过的源提交 |

- 止损条件
  - 触发状态：出现意外外站请求、合法 GET 失效、能绕过请求白名单、或测试基线与分支 SHA 不一致。
  - 期限：2026-10-23
  - 动作：停止 V1.2 工程执行，记录最小失败输入和 CI 日志，返回 Gate A 重新裁决；不回写 V1.1 原 Draft。

## 06｜审批与独立性

这份文本仅是**人工可审阅的 Gate A 候选**，不是来自 `de.py gate A` 的真实 token，也未收到用户对**这份具体候选**的单独确认。Gate B 的执行步骤见关联 [计划候选](../plans/V1_2_WINDOWS_LOOPBACK_HARDENING_GATE_B_CANDIDATE.md)，也未获批准。必须先在允许执行的独立 Git checkout 中，对真实固定版 Skill 运行 `de.py init --risk high`、`check card`、`gate A` 预览，由用户审阅并单独确认，然后再检查计划并预览 Gate B；不得自行 --confirm。

本模型是工程参与方，不能冒充独立 reviewer。**用户说「下一阶段」不等于看过新的两份 Gate 预览，更不等于授权发布。**
