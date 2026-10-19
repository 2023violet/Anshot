# P03-G：隔离剪贴板能力探测止损记录

本批没有通过隔离系统剪贴板验收。两次真实能力尝试均在创建 window station 时失败，按止损条件停止；没有提权、修改权限、复用已有 station 或回退读写交互桌面的剪贴板。完整产品继续开发，最近一份通过完整产品代码回归的局部基线仍为 [P03-F](../P03-F/README.md) 的固定 EXE/源码快照，不将其证据套到后续测试工具改动。

| 尝试 | 实际结果 | 访问范围 |
|---|---|---|
| 自动名称 + `CWF_CREATE_ONLY` | 创建失败，OS183：名称已存在 | 只查询原上下文句柄、辅助剪贴板序号 metadata；未打开新剪贴板，未读取或写入内容 |
| PID/时间唯一名称 + `CWF_CREATE_ONLY` | 创建失败，OS5：拒绝访问 | 同上；不提升当前 token、不修改成员身份/ACL/privilege |

隔离对象和桌面没有成功创建，因此上下文切换、空剪贴板断言和恢复路径均未实测。序号 metadata 不证明内容完整性；`0` 或相等也不能替代内容证据。完整格式读写、占用重试、实际 Ctrl+V 与 Paint/第二应用回贴均为 **NOT_RUN / NOT_VERIFIED**。

## 保留的代码与证据

- 显式测试工具为 `poc/native-egui/examples/clipboard_probe.rs`，当前只实现 `--isolation-only`。未加入正常 GUI 运行路径，也不作为产品剪贴板的新实现。
- `tests/native_oracle/run_clipboard_checks.py` 在独立预审身份匹配后构建，复制固定 EXE，再于执行前后重验源码/EXE；本批 Python 优化为 0。工具没有 WinSta0 fallback，也没有内容 Get/Empty/Set 调用。
- Native manifest 只增加测试用途的 windows-sys 0.61.2 dev 依赖边，根锁字节不变、已有包版本与校验和未升级。新 feature 中的 Security 仅满足创建 API 的参数类型；没有设置或修改任何安全描述符。
- 静态预审修正了 `CountClipboardFormats` 返回 0 的错误歧义，以及构建后/执行前缺少源码身份复查。新名称方案重新经过 genuine Gate A/B 与独立预审，未静默重试旧方案。
- 两份失败报告、EXE、日志、预审与源码快照均保留；精确身份见 [RESULTS.json](RESULTS.json)，独立复核见 [INDEPENDENT_REVIEW.md](INDEPENDENT_REVIEW.md)。这些是能力失败证据，不是成功验收记录。

本批只完成测试工具编译及限定能力尝试，未执行其后排定的完整格式/忙重试/窗口观察及全套回归；计划保持真实未完成状态，以“止损”归档。普通产品的图片变换、编辑和导出代码没有在本批改动；增加测试配置后的当前工作树不能仅凭旧 F 结果称为全面通过。

## 后续方向

保留真实系统剪贴板与跨应用验收要求，后续选择可证明保留用户现有内容的桌面测试方式，或另安排可用测试环境；不为运行这条探针修改系统权限。与此同时继续中文/字体、代表状态截图、小窗口、IME/Narrator、24MP/性能与包/许可等独立可逆工作。实际窗口观察使用 computer-use，不混用自建桌面输入控制程序。

genuine 工作区为 `target/workflows/p03-g`；代理按[用户批准政策](../../governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md)确认闸门并执行非删除归档。没有提交、推送、发布或删除用户/V1/历史资料。

依据：[Microsoft window station 与剪贴板](https://learn.microsoft.com/en-us/windows/win32/winstation/window-stations)、[CreateWindowStationW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createwindowstationw)。文档定义和本机失败实测分别记录。
