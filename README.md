# Agent Rules Manager

Agent Rules Manager 是一个 local-first 的桌面应用与 CLI：用多个 **Profile** 管理不同场景的规则，并把当前机器选中的 Profile 安全投射到 Codex、Claude Code、Grok、OpenCode、Qwen Code 等 coding agent。

规则正文仍由用户在 VS Code、Cursor、Typora 等工具中维护；本项目负责结构校验、有序渲染、本机切换、Agent 检测、原生路径接入、变更预览以及带漂移保护的回滚。

## 核心模型

- **Profile**：一组可同步的有序 Markdown 规则。每个 Profile 必须以自己的 `AGENTS.md` 开头，也可追加其他 `.md` 文件。
- **Runtime**：根据一个 Profile 生成的不可变 `AGENTS.md` 与来源清单，属于本机缓存。
- **`current`**：本机稳定入口，指向当前 Runtime。Agent 始终接入 `current/AGENTS.md`，切换 Profile 无需重新接入。
- **Adapter**：把稳定入口映射到不同 Agent 的原生规则路径。

```text
profiles/work/AGENTS.md ───────┐
profiles/work/rules/review.md ─┴─ render ── .runtime/work-<digest>/
                                                    ▲
                                                    │
                                       current ─────┘
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    v                     v                     v
                Claude link           Codex link         OpenCode link
```

- 同时检查命令和配置目录，避免桌面应用因 `PATH` 不完整漏掉已使用的 Agent。
- 接入使用指向 `current/AGENTS.md` 的符号链接；退出接入时会把当前规则落成普通文件，之后可独立修改。
- 已有普通文件是合法的“独立使用”状态；选择接入时会被视为冲突，绝不静默覆盖。
- 写入前必须生成计划；任一目标存在未托管文件或异常链接时，整次操作停止。
- 应用前持久化快照；回滚检测到应用后的人工修改时拒绝覆盖。

## 规则库布局

默认规则库位于 `~/.agent-rules`：

```text
~/.agent-rules/
├── schema.json
├── profiles/
│   ├── default/
│   │   ├── profile.json
│   │   ├── AGENTS.md
│   │   └── rules/
│   │       └── safety.md
│   └── work/
│       ├── profile.json
│       ├── AGENTS.md
│       └── rules/
│           └── review.md
├── .runtime/                    # 本机生成，不同步
│   └── work-8d9f20b751a4/
│       ├── AGENTS.md
│       └── manifest.json
└── current -> .runtime/work-8d9f20b751a4/  # 本机选择，不同步
```

规则正文只属于 Profile；渲染结果属于 Runtime；`current` 只表达这台机器当前选择。格式细节见 [Rule Library](docs/RULE_LIBRARY.md)。

## Profile 示例

`profiles/work/profile.json`：

```json
{
  "schemaVersion": 2,
  "id": "work",
  "name": "Work machine",
  "description": "Repository delivery and review conventions.",
  "instructions": [
    "AGENTS.md",
    "rules/review.md"
  ]
}
```

约束：

- `AGENTS.md` 必须存在且排在第一位；
- 补充文件必须是 Profile 目录内的唯一相对 `.md` 路径；
- 不允许绝对路径、`.`、`..` 或符号链接源；
- 渲染结果带 `<!-- Source: profiles/<profile>/<file> -->` 注释，便于定位来源。

## 多机器同步边界

项目固定了传输层边界，但尚未内置 Git push/pull：

| 参与同步 | 只保留在本机 |
| --- | --- |
| `schema.json` | `.runtime/**` |
| `profiles/**` | `current` |
| 用户选择纳入版本控制的普通说明文件 | 应用状态目录中的 `machine.json` |
|  | 投射与规则库回滚快照 |
|  | Agent 原生路径、凭据和登录状态 |

不同机器可以同步相同 Profile，却分别启用 `work`、`personal` 或其他 Profile。初始化只在 `.gitignore` 不存在时创建 `.runtime/` 与 `current` 忽略项，不覆盖已有文件。

## 安全与迁移

所有文件系统写入遵循相同事务边界：

1. 先生成可读计划，不写文件。
2. 未托管文件、异常链接、重复 id 或非法路径阻止完整操作。
3. 第一次修改前持久化每个目标的原始状态、期望摘要和本次新建目录。
4. 应用时重新检查原始状态，并校验每个写入结果。
5. 中途失败或显式回滚只恢复仍处于预期状态的目标。
6. 回滚只清理本事务创建且仍为空的目录；出现用户文件时保留目录和内容。
7. 已存在但内容不符的不可变 Runtime 会被视为冲突。

### 无 schema 的旧根文件

若只发现普通文件 `~/.agent-rules/AGENTS.md`，初始化会在精确备份后把内容迁入 `profiles/default/AGENTS.md`，创建默认 Runtime，删除旧根入口，并保留旧路径来源信息以识别尚未刷新的 Agent 链接。

### schema v1 升级

旧版 v1 的“规则包 + Profile 组合”会通过同一套预览和回滚事务升级到 v2：

- 每个旧 Profile 变成一个目录，并直接持有其原有有序规则副本；
- 文件内容逐字复制，旧规则只在所有目标副本准备完成后删除；
- 未被任何 Profile 使用的旧规则包会阻止升级，避免规则被遗落；
- 旧规则包目录中存在未声明文件或符号链接时会阻止升级；
- 本机仍启用同名 Profile，并重新生成 Runtime；
- 回滚可恢复 v1 文件、选择和目录结构。

## Agent 接入

| Agent | 原生路径 |
| --- | --- |
| Claude Code | `~/.claude/CLAUDE.md` |
| Codex | `~/.codex/AGENTS.md` |
| Grok | `~/.grok/AGENTS.md` |
| OpenCode | `$OPENCODE_CONFIG_DIR/AGENTS.md`、`$XDG_CONFIG_HOME/opencode/AGENTS.md` 或 `~/.config/opencode/AGENTS.md` |
| Qwen Code | `$QWEN_HOME/QWEN.md` 或 `~/.qwen/QWEN.md` |

Adapter 只处理 Agent 协议差异；同步的 Profile 内容保持 CLI 和 Provider 中立。检测不会读取凭据、会话或登录状态。应用状态和备份位于平台应用数据目录，不放进规则库。

## CLI

规则库写命令默认只输出计划，加 `--apply` 才执行：

```bash
# 初始化、旧根文件迁移或 v1→v2 升级
cargo run -p arm-cli -- init
cargo run -p arm-cli -- init --apply

# Profile
cargo run -p arm-cli -- profiles list
cargo run -p arm-cli -- profiles create work --name "Work"
cargo run -p arm-cli -- profiles create work --name "Work" --apply
cargo run -p arm-cli -- profiles add-file work rules/review.md
cargo run -p arm-cli -- profiles add-file work rules/review.md --apply
cargo run -p arm-cli -- profiles activate work
cargo run -p arm-cli -- profiles activate work --apply
cargo run -p arm-cli -- profiles rollback

# Agent 原生路径
cargo run -p arm-cli -- status
cargo run -p arm-cli -- plan
cargo run -p arm-cli -- apply --agents codex,claude
cargo run -p arm-cli -- plan --agents qwen --disconnect
cargo run -p arm-cli -- apply --agents qwen --disconnect
cargo run -p arm-cli -- rollback
```

所有命令支持 `--root <directory>` 与 `--state-root <directory>`；查询和计划支持 `--json`。

## 桌面开发

需要 Bun、Rust stable 及 [Tauri 2 平台依赖](https://v2.tauri.app/start/prerequisites/)：

```bash
bun install --frozen-lockfile
bun run test
bun run build
cargo test --workspace
bun run tauri dev
```

桌面端只允许用固定打开目标处理后端重新校验过的 Profile 文件；WebView 不能提交任意可执行文件或绝对路径。普通 `bun run dev` 使用纯内存演示数据，不读取或修改真实用户目录，也不会启动本地应用。

## 项目结构

```text
crates/arm-core/   Profile 校验、渲染、事务投射与回滚
crates/arm-cli/    CLI
src-tauri/         Tauri 命令与安全的外部打开边界
src/               React 控制台与惰性浏览器演示
docs/              数据格式与架构契约
```

实现约束见 [架构说明](docs/ARCHITECTURE.md)，参与方式见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## License

[MIT](LICENSE)
