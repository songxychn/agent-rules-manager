# 仓库规则

Agent Rules Manager 是 local-first 的配置工具。把用户的说明文件当作高价值数据。

## 安全不变量

- 绝不覆盖未托管的普通文件。
- 每次文件系统变更都必须先预览再应用。
- 备份每个被改写的目标；回滚时若发现应用后的漂移，必须拒绝覆盖。
- 权威规则源对 CLI 保持中立；各 Agent 的差异放进适配器。
- 不要把凭据、聊天记录或 Provider 登录状态放进托管规则库。

## 验证

- 改 Rust 后运行 `cargo fmt --all`、`cargo clippy --workspace --all-targets --locked -- -D warnings` 和 `cargo test --workspace`。
- 改前端后运行 `bun run test` 和 `bun run build`。
- 保持浏览器演示数据可用，界面工作不必改写真实主目录文件。
