# 治理调整草稿（2026-10-19 用户确认，已写入 AGENTS.md）

日期：2026-10-19　基线：`c7919bc`，本地 tag `pre-redesign-2026-10-19`

## 拟替换 AGENTS.md 的内容要点

1. **项目定位**：ShareReady（临时代号）进入产品重新定义阶段。目标是以 Rust 为可信主导的 Windows 桌面图像工具；产品范围扩大（截图/标注/遮挡等，以 `ui-design/01_PRD.md` 为准）。V1.3 与 `poc/native-egui` 冻结为历史参考，不再新增功能。
2. **工作流**：采用《AI 驱动的产品 UI 设计工作流 v1.0》的 01–07 阶段与 Gate 01–07，交付物放在 `ui-design/`。每个 Gate 由用户确认后记录在对应文件末尾。
3. **Rust 可信边界（不变）**：图像解码、像素变换、遮挡、元数据剥离、导出、保存与剪贴板交付只经过 Rust；UI 层同样使用 Rust 原生框架（框架在 Gate 03 选定）。5 张冻结金样本必须持续通过。
4. **证据（减负）**：每个里程碑（Gate）一份简短证据：完整 SHA、命令与退出码、结果、已知限制。取消每批多份 pre-review / R1 / R2 文档。独立审查只在 Gate 03、06、07 各做一次，由单独子代理执行，如实标注为代理审查。
5. **状态用语（不变）**：DOCUMENTED / MEASURED / HYPOTHESIS / NOT_VERIFIED；不把机器结果写成真人验收。
6. **安全边界（不变）**：main-only；不改写历史、不强推、不删除用户文件或 V1/POC 目录、不推送、不发布、不对外联系，均需用户单独确认。
7. **deciding-and-executing Skill / `.decision/` 令牌流程**：由上述 Gate 01–07 替代，不再要求每批生成 Gate A/B 令牌。

## 需要你确认
- 同意后我把以上内容写入 `AGENTS.md`，并在 `README.md`、`docs/V2/README.md` 顶部注明“产品重新定义中，见 ui-design/”。
- 旧的 `docs/governance/AUTONOMOUS_LOCAL_DEVELOPMENT.md` 保留为历史记录，不删除。
