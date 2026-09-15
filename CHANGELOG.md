# 更新日志

本文记录 Agent Rules Manager 的公开变更，格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

`0.x` 仍是预发布：安装包形态、适配器覆盖和配置格式都可能调整。

## [未发布]

## [0.1.0] - 2026-09-16

### 新增

- 桌面安装包：macOS DMG（Apple Silicon / Intel）、Windows 当前用户 NSIS、Linux `.deb` 与 AppImage。
- 桌面端可在设置里检查 GitHub Releases 并一键安装更新；启动时若有新版本会提示。浏览器演示不会下载安装包。
- 用多个 Profile 管理规则，每个 Profile 的规则正文固定写在一个 `AGENTS.md` 中（规则库 schema v3）。
- 生成本机 Runtime，经 `current` 符号链接投射到 coding agent 的原生规则路径。
- 接入前必须预览；已有普通文件需明确确认后才会备份接管，异常链接等硬冲突会整次停止。
- 应用前持久化快照；回滚发现应用后的人工修改时拒绝覆盖。
- 操作历史与安全恢复预览，可按记录恢复并在失败时保留补偿材料。
- 浏览器内存演示，不读写真实规则库或 Agent 配置。
- CLI（`arm-cli`）覆盖初始化、Profile、接入、回滚；查询和计划支持 JSON。
- 中英界面。

当前已核对的客户端入口见 [README](README.md#agent-接入)。

### 变更

- CI 增加 Ubuntu 测试，以及 rustfmt、clippy 检查。

### 已知限制

- Apple 公证和 Windows Authenticode 尚未配置。未公证的 macOS 应用第一次打开需要按住 Control 点击。
- Windows 接入 Agent 仍需要[开发人员模式](https://learn.microsoft.com/windows/apps/get-started/enable-your-device-for-development)（或管理员权限）才能创建符号链接。
- 桌面端以 macOS 为主要验证环境；Linux 与 Windows 安装包会随 Release 构建，但完整文件接入和回滚尚未建立发布验证矩阵。
- 尚未内置 Git push/pull；同步边界已写进规则库 schema，传输仍交给系统 Git 或其他工具。
- 应用内更新依赖公开仓库上的非预发布 GitHub Release 与 `latest.json`。
- 不管理 Skills、MCP、子代理、凭据或模型登录状态。
- 所有已接入项目跟随本机当前 Profile，不支持每个项目独立选用 Profile。

[未发布]: https://github.com/songxychn/agent-rules-manager/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/songxychn/agent-rules-manager/releases/tag/v0.1.0
