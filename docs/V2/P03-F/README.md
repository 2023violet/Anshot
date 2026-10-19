# P03-F：有界图片粘贴与复制恢复

本批为原生工作台接入图片粘贴，并强化复制的当前成品检查和失败恢复。前批新文件保存见 [P03-E](../P03-E/README.md)。当前完成本地代码、合成/自有内存测试和版本取证；真实系统剪贴板操作、Paint 与第二应用回贴尚未执行，不能记为完整 M04/M06 或 G06 通过。

## 产品行为与格式规则

- `Paste image (Ctrl+V)` 进入同一个受限读取入口。注册 PNG 优先，其次 CF_DIBV5、CF_DIB。没有图像、剪贴板忙、超限、坏图或不支持的格式均提示恢复办法，保留源、修订、历史和当前 PNG，不自动读取文本路径、不无限重试。
- Windows 平台在复制前检查 HGLOBAL 大小：PNG 最多 25 MiB，DIB 最多 `24,000,000 × 4 + 124` 字节。锁定后复制，先解锁再关闭剪贴板，不释放系统所有权。此限制约束应用内读取与后续解码，不能约束系统提供句柄前的延迟渲染、格式合成或阻塞。
- PNG 分支必须真有 PNG 签名，且通过既有 Rust Core 解码；有 PNG 但无效时直接失败，不默默降级另一图像。独立审查发现并用冻结 JPEG 复现了“PNG 标签中 JPEG 被通用解码器接受”的问题；修复后重新固定了 EXE。
- Rust Core 支持常见 packed DIB：40/108/124 字节头，24 位 BI_RGB、32 位 BI_RGB 或标准 BGR 位掩码，正确处理上下方向及行填充。只在明确 alpha 掩码存在时保留 alpha；32 位 BI_RGB 的高字节按未使用处理为不透明，不猜测透明度。
- 非 sRGB/Windows sRGB、嵌入或链接 profile、调色板、压缩和非标准掩码等格式明确拒绝，不访问 profile 路径。尺寸、行跨度和完整数据边界在像素分配前校验，最多 24MP；新编码候选 PNG 仍受 25 MiB 输入上限约束。整体峰值资源/24MP 性能另行验收。
- 文件和图片粘贴共用候选解码、显示准备和丢弃确认。已有编辑、Redo 或手势时先等待；取消保留当前源/PNG/历史，仅明确放弃后替换。trace 模式与已有确认框在读取前阻止粘贴。
- egui-winit 0.36.2 会在 KeyDown 吞掉 Ctrl+V 并只读文本，因此新增一个隔离的 Windows 输入桥接 crate，在消息进入 egui-winit 前识别本应用前台窗口的 Ctrl+V。文本焦点交给原生文本粘贴；重复按下不重复请求图片，其他窗口及 Alt 组合不拦截。界面下一帧仍检查文本焦点、trace 与确认框。消息结构测试和源码核查尚不是物理按键端到端观察。
- 复制先验证操作类型、当前凭证和确认状态，再把当前 Rust PNG 交给接收函数。失败保留成品和历史，提示重试或 `Save new PNG`；过期/错误操作不调用接收函数、不更新新文档状态。真实路径继续使用已有 arboard 图像写入，只报告系统接受，不承诺跨应用格式或透明度。

## 验证与身份

精确输入、锁、EXE、命令和 PNG 证据见 [RESULTS.json](RESULTS.json)，独立审查见 [INDEPENDENT_REVIEW.md](INDEPENDENT_REVIEW.md)。主分支/HEAD 保持，依赖锁只增加必要输入桥接 path 包和 native 依赖边；没有既有包版本/校验和升级，根锁字节保持前批不变。GUI/Editor 的 `forbid(unsafe_code)` 保留；新 Core DIB 模块也禁止 unsafe，FFI 仅在薄 Windows/输入桥接边界。

| 检查 | 范围 |
|---|---|
| Core 5 + DIB 7 项 | 保留既有 Core；DIB 全图通道、方向、padding、alpha、格式、尺寸与截断 |
| Windows 平台 12 项 | 前批保存 9 项，加自有 HGLOBAL 精确复制、上限/重试、无效句柄 3 项；不读写当前系统剪贴板 |
| 原生应用 41 项 + 桥接 4 项 | 候选/丢弃确认、坏格式、PNG/JPEG 混淆、DIB alpha、文本焦点、受控复制失败/过期与窗口消息条件 |
| 主 workspace 53 项 | 原 43 项加 DIB 7、自有内存 3；含平台与 Core，不将它们重复叠加为独立总数 |
| 独立 PNG 检查器 19 项 | 实际重跑；新 EXE 保存输出与 DX12 / GTX 1650 窗口 Review 输出均作独立全图/结构检查 |
| V1 五金样 | 重新构建当前共享 Core 的 release CLI 与 preview；5 张冻结输入逐像素回归通过，不更新金样。JPEG 遵守原合同，使用冻结 Rust 归一源与 Pillow 独立合成/输出检查，并单列与 Pillow 源解码差异 |
| 独立额外探针 | 288 组 DIB 尺寸/方向/格式/alpha 全图比较、10,000 组小型畸形头未 panic；不是全面模糊测试或安全审计 |

`tests/golden/verify_v1_baseline.py` 仅补齐 Windows 的两处 `.exe` 路径；冻结合同不变。Windows CI 测试命令同步为 native `--workspace` 以运行输入桥接测试；远程 CI 未执行。测试工具仍使用历史 `target/p03-c-evidence/native-<时间>/` 前缀，版本以本批源码、build ID、EXE 和报告哈希识别。

## 仍待完成

本轮没有自动读取/替换用户当前剪贴板，没有修改系统历史/云同步/隐私设置。自有内存复制、受控 OS 接收函数和 MSG 测试分别只证明对应边界，不能代替真实 Ctrl+V、系统忙冲突、真实复制及跨应用回贴；尤其 alpha 和方向仍须在 Paint 与第二应用分别记录。

后续继续真实合成图片剪贴板链路、中文/IME/Narrator、小窗口/代表状态截图、混合 DPI/双屏、24MP 与性能、离线包/许可/剩余 CI、安全与普通人任务。完整产品、入口切换、V1 退出和发布均未完成。genuine Gate A/B/review 由代理按[明确批准政策](../../governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md)记录，不称人类逐 token 审阅；不推送、不发布、不清理用户文件或历史工作区。

设计参考：[Microsoft 剪贴板格式](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats)、[BITMAPV5HEADER](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapv5header)、[GetClipboardData](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboarddata)。参考定义不替代本机测量。
