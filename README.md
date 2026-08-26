# Agent Rules Manager

把一份中立的 `AGENTS.md` 安全投射到多个 coding agent 的原生规则路径。

Agent Rules Manager 是一个 local-first、开源的桌面应用与 CLI。它面向已经同时使用 Codex、Claude Code、Grok、OpenCode、Qwen Code 等工具的人：规则内容可以只维护一份，也可以让某个 Agent 继续持有独立文件；本工具负责检测已安装 Agent、预览接入选择并提供可审计回滚。

桌面界面支持跟随系统、简体中文和 English，语言偏好只保存在本地 WebView，不会改写规则正文或 Agent 配置。

> 当前状态：`0.1.0` MVP。核心文件事务、CLI 和 Tauri/React 控制台已经成形，尚未发布安装包。

## 为什么不是 dotfiles 工具

chezmoi、yadm 擅长把文件同步到多台机器；Agent Rules Manager 关注的是同一台机器上不同 Agent 的规则协议：

- 同时检查命令是否可用以及配置目录是否存在，避免桌面应用因 `PATH` 不完整漏掉已使用的 Agent。
- 接入的 Agent 使用指向规范源的符号链接；退出接入时把当前规范规则落成普通文件，之后可独立修改。
- 已有普通文件属于合法的“独立使用”状态；选择接入时会被视为需人工处理的冲突，绝不静默覆盖。
- 写入前生成只读计划；任一选中目标存在异常链接或受保护的独立文件时，整次应用被阻止。
- 应用前持久化文件快照；回滚时若检测到应用后的人工修改，则拒绝覆盖。

## 默认路径

规范规则库固定使用 `~/.agent-rules/AGENTS.md`。程序不会探测或沿用其他历史路径；如需为测试或特殊环境指定不同目录，可显式设置 `AGENT_RULES_HOME` 或使用 CLI 的 `--root`。

| Agent | 原生路径 | 接入方式 |
| --- | --- | --- |
| Claude Code | `~/.claude/CLAUDE.md` | 可选符号链接 / 独立文件 |
| Codex | `~/.codex/AGENTS.md` | 可选符号链接 / 独立文件 |
| Grok | `~/.grok/AGENTS.md` | 可选符号链接 / 独立文件 |
| OpenCode | `$OPENCODE_CONFIG_DIR/AGENTS.md`、`$XDG_CONFIG_HOME/opencode/AGENTS.md` 或 `~/.config/opencode/AGENTS.md` | 可选符号链接 / 独立文件 |
| Qwen Code | `$QWEN_HOME/QWEN.md` 或 `~/.qwen/QWEN.md` | 可选符号链接 / 独立文件 |

默认 adapter 目录是受支持 Agent 的检测目录。检测结果只决定界面是否提供接入开关，不读取凭据、会话或登录状态。旧版 Claude 托管 include 会被识别为“旧版接入”；退出时会把中央规则复制进原文件并保留 include 周围的 Claude 专属内容。

应用自身的投射快照和状态不放在规则库里。macOS 使用 `~/Library/Application Support/agent-rules-manager`，Linux 使用 `$XDG_STATE_HOME/agent-rules-manager` 或 `~/.local/state/agent-rules-manager`。

## 安全模型

`apply` 遵循固定顺序：

1. 重新检查规范源、安装信号和所有发生接入变化的目标。
2. 任一目标存在受保护的独立文件或异常链接时，整次应用不写入。
3. 把所有目标的原始状态先持久化到本地快照。
4. 执行 adapter 写入；中途失败时自动恢复已处理目标。
5. `rollback` 只恢复仍处于“刚应用状态”或“原始状态”的目标，检测到其他漂移则停止。

规范 `AGENTS.md` 是用户拥有的内容。桌面应用只读取其路径、摘要和修改时间，并将编辑工作交给用户选择的外部软件；投射回滚不会覆盖或恢复规范源正文。

项目不会管理凭据、会话记录或 Agent 登录状态。

## CLI

需要 Rust stable toolchain：

```bash
cargo run -p arm-cli -- status
cargo run -p arm-cli -- plan
cargo run -p arm-cli -- apply --agents codex,claude
cargo run -p arm-cli -- plan --agents qwen --disconnect
cargo run -p arm-cli -- apply --agents qwen --disconnect
cargo run -p arm-cli -- rollback
```

构建后可直接使用 `agent-rules`。所有读写命令均支持 `--root <directory>` 和 `--state-root <directory>`；`status`、`plan`、`apply`、`rollback` 支持 `--json`。

## 桌面开发

需要 Bun 1.4+、Rust stable 以及 [Tauri 2 的平台依赖](https://v2.tauri.app/start/prerequisites/)：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

项目内的 Tauri 启动脚本会自动把标准的 `~/.cargo/bin` 加入子进程 `PATH`，因此安装后不必为了当前终端额外执行 `source ~/.cargo/env`。

```bash
bun install --frozen-lockfile
bun run test
bun run build
bun run tauri dev
```

桌面控制台会检测当前系统中可用的常见编辑器，并通过右上角“上次选择 + 下拉切换”的紧凑分体按钮打开规范 `AGENTS.md`。Finder / 文件资源管理器用于显示文件，终端目标打开规则库目录；前端只提交固定目标 id，实际路径和启动参数由 Tauri 后端决定。

普通 `bun run dev` 会运行浏览器演示数据，不读取或修改真实用户目录，也不会启动本地应用；只有 Tauri 后端会执行文件检查和外部打开操作。

## 项目结构

```text
crates/arm-core/   文件检测、计划、事务应用与回滚
crates/arm-cli/    无界面 CLI
src-tauri/         Tauri 2 命令边界
src/               React 控制台与浏览器演示模式
docs/              架构和扩展约定
```

路线图包括规则包、机器/工作场景 profiles、可配置 adapters、受控导入现有独立规则和签名发布。实现约束见 [架构说明](docs/ARCHITECTURE.md)，参与方式见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## License

[MIT](LICENSE)
