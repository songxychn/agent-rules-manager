# 贡献指南

感谢帮助改进 Agent Rules Manager。

当前是 `0.x` 早期版本。桌面安装包、浏览器演示和源码开发说明见 [README](README.md#安装)。浏览器演示使用内存数据，适合作为界面改动的起点。

## 开发

1. 行为或安全模型变更请先开 [Issue](https://github.com/songxychn/agent-rules-manager/issues/new/choose)。
2. Profile 规则正文保持对 Provider 中立；各 Agent 的差异放进适配器。
3. 每个文件系统边界情况都要补回归测试。
4. 提交 pull request 前运行 `cargo fmt --all`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace`、`bun run test` 和 `bun run build`。CI 会在 macOS 与 Ubuntu 上跑测试，并强制 rustfmt / clippy。

会静默覆盖未托管文件、不经可预览计划就写入、同步本机状态，或绕过回滚漂移检查的改动不会被接受。接管已有的 Agent 普通文件必须先明确确认，并在替换前留下完整可读备份；指向外部的符号链接和不受支持的条目仍是硬冲突。

测试请使用临时的主目录、规则库和状态目录，禁止写入开发者真实的 Agent 配置路径。

缺陷报告请使用 Issue 模板，写明操作系统、相关工具版本、复现步骤和脱敏后的计划输出。不要附带真实规则库、凭据或主目录内容。漏洞报告请遵循 [安全策略](SECURITY.md)。
