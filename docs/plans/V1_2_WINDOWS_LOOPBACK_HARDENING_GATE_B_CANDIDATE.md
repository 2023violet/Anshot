# ShareReady V1.2｜Windows Loopback 安全加固 · Gate B 执行计划候选

> 2026-10-16｜**PLAN CANDIDATE / GATE A & B BOTH NOT CONFIRMED**；此文只提供步骤与验收命令，不授权修改代码。
> 依赖：[V1.2 Gate A 决策候选](../decisions/V1_2_WINDOWS_LOOPBACK_HARDENING_GATE_A_CANDIDATE.md)。
> 原始 source HEAD：`c16b0953a4705a36b5c93a1645704ebd7a4cde7e`；固定 Skill SHA：`1eb9624ee2037e4a2b253ee8120fa274dad1ef21`。**绝不继承 E1 项目的旧 Gate token。**

## 计划摘要

- 高风险档：**high**（本地服务攻击面 / 安全配置）。
- 工程起点：**另起候选工程分支**，基于 PR #14 固定 SHA；保留既有 `feature/v1-1-windows-launcher-edit-ux` 不变；先红测，再最小修复，再完整回归，最后独立审查。
- 第一可执行步骤（待新 Gate A/B 真实确认）：V12-01 原生 Windows HTTP 负向探针，先验证现有失败反例。禁止自签审查。

## 6 步可验收计划

| ID | 交付与验证判据 | 依赖 | 对应 AC | 风险 |
| --- | --- | --- | --- | --- |
| V12-01 | 在隔离工程分支新增原生 PowerShell 原始 TCP 协议测试（仅模拟静态请求、不包含用户图片）。证明**旧实现**不具备 AC02/AC03，不绕过测试；保存最小失败输入、SHA 和日志 | 无 | 2,3 | 可逆 |
| V12-02 | 最小修正本地 HTTP 解析：Host 必需、仅唯一准确的 IPv4 loopback + 动态端口；限制请求行、Header 行和累计上界；EOF/超时 fail closed。禁止改 Rust/WASM 核心 | V12-01 | 2,3,4 | 可逆 |
| V12-03 | Windows 10/11 机器上完整跑固定路由、正常 GET/HEAD、POST=405、Path 404、HTTP CSP、端口仅 loopback；并通过新增 Host/长度/EOF 负向 TCP 测试 | V12-02 | 1,2,3,4,5 | 可逆 |
| V12-04 | Linux headless Chromium 真实 Rust/WASM、JPEG EXIF、PNG 全遮挡像素、Clipboard/网络/CSP 正向回归；仅合成素材，生成同 commit 工件与 SHA | V12-02 | 1,4,5 | 可逆 |
| V12-05 | 独立 review：比较 V1.1 与候选每条变化、负向 HTTP 样本、不覆盖的 DoS 和发布隐私缺口；若漏洞/测试盲点存在则 REWORK | V12-03,V12-04 | 5,6 | 可逆 |
| V12-06 | 文档知识同步：Windows 用户指南、支持范围、已证实/未证实声明、CI 工件索引，确认两个原 Draft 未被修改或合并；仅将候选 PR 保持 Draft | V12-05 | 1,6 | 可逆 |

### 每一步可执行验证命令（**仅预览，尚未授权执行**）

命令必须在隔离 checkout 的仓库根目录执行；Windows 特定命令必须在 Windows 原生 runner，Linux cargo/Chrome 使用已有 GitHub Actions。真正使用时，应将下列命令逐条写入经 Skill 初始化的 plan.json 的 `verify_cmd`，以 `de.py verify ID` 保存当次代码指纹；GitHub CI 绿灯只代表真实运行时覆盖的步骤。

```text
V12-01: pwsh -NoProfile -File web-app/tests/test_windows_launcher.ps1
        + 新增独立 raw-TCP negative tests (预期旧实现失败，记录 RED 证据)
V12-02: pwsh -NoProfile -File web-app/tests/test_windows_launcher.ps1
        + 新增 raw-TCP negative tests 必须 GREEN
V12-03: GitHub Actions windows-localhost-launcher-smoke (native Windows 10/11 compatible runner)
V12-04: node --check web-app/app.js
        cargo test --manifest-path e1-spike/Cargo.toml --locked --offline
        cargo test --manifest-path e1-spike/Cargo.toml --locked --offline --test codec_security
        python3 web-app/tests/test_local_app.py  [需先编译真实 Rust WASM 并设置 Chrome 依赖]
V12-05: 人工独立审查报告，准确 SHA/环境/逐条证据/反对意见 (不可伪写 shell success)
V12-06: python3 <固定 closing-out skill>/scripts/audit.py --base feature/v1-1-windows-launcher-edit-ux
        + README/AGENTS 来源事实核对；不得删除分支或发布
```

> 以上带“+”的两项是明确的**测试合同占位项而非可执行命令**，实际 Gate B 不得直接带入未替换的字符串，否则校验失真。待高风险 Gate A 确认后，由执行者将完整真实路径/新测试文件和受控 Shell 命令填入当前 `de.py`，预览 Gate B 给用户看，审批后才能使用。V12-05 不得把同一 Agent 的审查意见作为独立 PASS。

## 首先要求 RED 的负向 TCP 用例

| Case | 原始请求特征 | 预期 |
| --- | --- | --- |
| N01 | 合法 GET、Host = `127.0.0.1:<boundPort>` | 200，完整静态资源 |
| N02 | Host = `attacker.example:<boundPort>` | 4xx，不返回 HTML/JS/WASM |
| N03 | HTTP/1.1 无 Host | 4xx |
| N04 | 两个 Host，第二个与第一个相冲突 | 4xx |
| N05 | Host = `localhost:<boundPort>` 或外部 IP | 4xx（V1.2 限定唯一访问 URL 为 IPv4 loopback） |
| N06 | HEADER 行 > 4 KiB / 总 Header > 16 KiB | 4xx 或关闭，不返回静态资源 |
| N07 | 请求行 > 2 KiB | 4xx 或关闭；报告调用时间与内存，检测读取前巨大字符串分配风险 |
| N08 | Header 缺失终止空行（EOF） | 不返回 200/资源 |
| N09 | POST /app.js、GET /secret.txt | 405 / 404（原有门禁不能退化） |
| N10 | 只监听 `127.0.0.1` + HTTP CSP 包含 `default-src 'none'` | PASS，检查真实绑定与响应头 |

防误报：N06/N07 的“超长流读取前不分配”仅凭返回 400 **不能证明**，需要资源边界的定量指标或改为定长分段解析才能成立。测试必须以严格的网络原始字节形式构造，不能依赖 `HttpWebRequest` 自动修复重复 Host 等畸形输入。

## 差异限制与风险预案

- 允许工程路径：`web-app/Start-ShareReady.ps1`、`web-app/tests/test_windows_launcher.ps1`、专门的 HTTP 负向测试、新的专属 CI workflow、`web-app/README.md`；其他路径需要先 replan + Gate B 重新确认。
- 所有步骤均只在隔离分支和合成数据完成；不得修改 `main`、V1/V1.1 原 Draft、E1/E2 工程分支、任何权限或发布路径。
- 如果头部检查影响正常浏览器 → 停止，保留 RED 实验输入，先调查是否需要支持 HTTP/1.0。
- 如果 Windows CI 与 Linux 功能测试冲突 → 先识别是否源提交/包装工件错配，不降低断言。
- 如果新测试被强制关闭服务而产生不稳定 → 区分功能回归与测试错误，不靠 timeout 放宽伪造通过。
- 如果发现远端传输图片、文件遍历、HTML CSP 失效 → STOP，另开高风险决定。

## 事前验尸与当前状态

1. CI 只在 Windows GitHub Hosted Runner PASS，未覆盖普通用户设备和杀软 → 保留 **NOT_VERIFIED**。
2. DNS rebinding 仍可能经浏览器的特殊网络策略变化 → 明确对真实浏览器的威胁模型，不能把 Host 加固宣称为“已消除全部攻击”。
3. 即使 V1.2 全绿，E1e R10 独立评审和 Stage 01 A1 真人用户研究仍开放 → **V1.2 工程候选不能变成 Release GO**。

**当前状态：Stage ① Gate A 候选 + Stage ② 预案草案完成；Gate A 未确认，Gate B 未预览/确认，V12-01..06 均未执行。** 必须先展示真实固定 Skill 的两步确认预览；不得在本仓库手工制造被脚本标记 DONE 的 `.decision/`。
