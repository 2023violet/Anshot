# E2｜工作台交互与真实 Rust/WASM 集成范围（工程候选）

日期：2026-10-15。分支：feature/e2-workbench-integration；起点：E1 修复后提交 7a234b957c177156389fcb3b2e668a58aa357c0c。

### 开发目标

把已认可的浅色 Precision Workbench 视觉方向连接到 E1 真正的 Rust/WASM PNG 遮挡接口，形成可在本地浏览器验证的单图操作链路：导入 → 框选/移动/修改 → 撤销/重做 → 实际 PNG 成品复核 → 复制/下载。

### 隔离条件

- 此分支是 E2 工程候选，不修改原 E1 的 Gate A/B，也不将 E1e 独立安全审查从 TODO 改为 DONE。
- 为避免在独立代码审核前碰真实私人数据，图片入口对 SHA256 采用显式白名单，只允许冻结合成素材 T01/T02/T04/T05。禁止其他图片/云传输。
- 真正的 PNG bytes 来自 E1 Rust 核心 redact_web；编辑中覆盖层仅供交互视觉反馈。成品复核前必须成功编码且再次解码输出。
- 跨浏览器 Clipboard paste-back 尚未充分验证，成功文案仅声称写入，不声称实际回读一致。
- 用户价值假设 A1 OPEN；原 Stage 01 S05a BLOCKED_BY_RECRUITMENT。无 main 修改、PR 合并、部署、发布。

### 范围

实现 HTML/CSS 工作台（顶栏、工具轨、画布、右 Inspector、成品复核），合成图片本地文件选择/拖入/粘贴，添加多矩形、选取移动、右下角调整、区域列表同步、Undo/Redo、Fit/缩放与平移、输出 revision 防过期、复制拒绝及下载回退。

### 验收

1. Node JS 语法、Python 测试脚本语法。
2. 原 E1 Rust cargo test 及 codec_security 离线测试。
3. 固定 Rust/WASM 包与 E2 独立页面在真实 Chrome 中运行。
4. 四个冻结 fixture 由实际鼠标拖出的矩形处理；四份 PNG 完整逐像素敏感区遮挡、预定保留区检查和 EXIF 清理通过。
5. 未知图片被拒绝；没有区域时禁止进入复核；Undo/Redo 正确；编辑后旧输出字节失效。
6. Clipboard 拒绝时提示实际失败，仍允许下载；网络请求均限 localhost/blob。
7. 提交真实运行工件、逐项报告以及独立审查阻断，不伪称完成产品安全认证。
