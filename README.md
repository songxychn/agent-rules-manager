# Agent Rules Manager

Agent Rules Manager 是一个 local-first 的桌面应用与 CLI：用多个 **Profile** 管理不同场景的规则，并把当前机器选中的 Profile 安全投射到 Codex、Claude Code、Cursor、Antigravity、Copilot 等 coding agent。

规则正文仍由用户在 VS Code、Cursor、Typora 等工具中维护；本项目负责结构校验、规则生成、本机切换、Agent 检测、原生路径接入、变更预览以及带漂移保护的回滚。

## 核心模型

- **Profile**：一套可同步的规则，全部正文固定写在该 Profile 的一个 `AGENTS.md` 中。
- **Runtime**：根据一个 Profile 生成的不可变 `AGENTS.md` 与来源清单，属于本机缓存。
- **`current`**：本机稳定入口，指向当前 Runtime。Agent 始终接入 `current/AGENTS.md`，切换 Profile 无需重新接入。
- **Adapter**：把稳定入口映射到不同 Agent 的原生规则路径。

```text
profiles/work/AGENTS.md ── generate ── .runtime/work-<digest>/
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
│       └── AGENTS.md
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
  "schemaVersion": 3,
  "id": "work",
  "name": "Work machine",
  "description": "Repository delivery and review conventions."
}
```

约束：

- 每个 Profile 固定使用一个 `AGENTS.md`，必须是普通 UTF-8 文件，不能是符号链接；
- `profile.json` 仅保存名称、说明等元数据，不再配置文件列表；
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

### schema v1 / v2 升级

旧版 v1 的“规则包 + Profile 组合”和 v2 的多文件 Profile，通过预览和回滚事务升级到 v3：

- 每个 Profile 的规则按旧顺序合并到一个 `AGENTS.md`，仅在文件之间补充分隔换行，保留各段原文；
- 原本只有一个文件时，正文逐字保持不变；
- 原文件和配置先备份，再移除已合并的补充文件；v2 的空补充目录同时清理；
- v2 中未登记的空目录会原样保留，不阻止升级；未声明文件、符号链接或未使用的 v1 规则包仍会阻止升级；
- 保留本机启用的 Profile 并更新规则；从未在本机启用的规则库不自动选择 Profile；
- 如需更新已启用的规则，规则更新和结构迁移会分别保存快照，可依次回滚；回滚拒绝覆盖之后的人为修改。

## Agent 接入

客户端目录同时用于 Rust 适配器和浏览器演示，当前包含 22 个客户端入口。全局视图默认合并 Gemini CLI / Antigravity 的共享文件，因此显示 17 个目标；项目视图显示 4 个目标。

| Agent | 范围 | 默认原生路径 |
| --- | --- | --- |
| Claude Code | 全局 | `~/.claude/CLAUDE.md` |
| Codex | 全局 | `~/.codex/AGENTS.md` |
| Grok | 全局 | `~/.grok/AGENTS.md` |
| OpenCode | 全局 | `~/.config/opencode/AGENTS.md` |
| Qwen Code | 全局 | `~/.qwen/QWEN.md` |
| [Gemini CLI](https://geminicli.com/docs/cli/gemini-md/) | 全局 | `~/.gemini/GEMINI.md` |
| [Antigravity](https://antigravity.google/docs/rules-workflows) | 全局 | `~/.gemini/GEMINI.md` |
| [GitHub Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions) | 全局 | `~/.copilot/copilot-instructions.md` |
| [Windsurf / Cascade](https://docs.devin.ai/desktop/cascade/memories) | 全局 | `~/.codeium/windsurf/memories/global_rules.md` |
| [Cline (VS Code)](https://docs.cline.bot/customization/cline-rules) | 全局 | `~/Documents/Cline/Rules/agent-rules.md` |
| [Cline CLI](https://docs.cline.bot/cli/cli-reference) | 全局 | `~/.cline/data/settings/rules/agent-rules.md` |
| [Roo Code](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/docs/docs/features/custom-instructions.md) | 全局 | `~/.roo/rules/agent-rules.md` |
| [Kilo Code (legacy extension)](https://github.com/Kilo-Org/kilocode-legacy/blob/main/docs/legacy-ides/customize/custom-rules.md) | 全局 | `~/.kilocode/rules/agent-rules.md` |
| [Kiro](https://kiro.dev/docs/steering/) | 全局 | `~/.kiro/steering/agent-rules.md` |
| [Cursor](https://cursor.com/docs/context/rules) | 项目 | `<project>/AGENTS.md` |
| [GitHub Copilot (IDE)](https://docs.github.com/en/copilot/reference/custom-instructions-support) | 项目 | `<project>/.github/copilot-instructions.md` |
| [Continue](https://docs.continue.dev/customize/deep-dives/rules) | 项目 | `<project>/.continue/rules/agent-rules.md` |
| [Amp](https://ampcode.com/docs/customize/agents-md) | 全局 | `~/.config/amp/AGENTS.md` |
| [Factory Droid](https://docs.factory.ai/harness/agents-md) | 全局 | `~/.factory/AGENTS.md` |
| [Augment / Auggie](https://docs.augmentcode.com/setup-augment/guidelines) | 全局 | `~/.augment/rules/agent-rules.md` |
| [Zed Agent](https://zed.dev/docs/ai/instructions) | 全局 | `~/.config/zed/AGENTS.md` |
| [Junie](https://junie.jetbrains.com/docs/junie-plugin-project-settings.html) | 项目 | `<project>/.junie/AGENTS.md` |

Adapter 只处理 Agent 协议差异；同步的 Profile 内容保持 CLI 和 Provider 中立。检测不会读取凭据、会话或登录状态。应用状态和备份位于平台应用数据目录，不放进规则库。

若 Agent 原生路径已有常规文件，接入计划不会再直接阻止启用，而会标记为“需确认接管”。确认后，程序先把完整旧文件保存到应用状态目录的 `backups/originals/<快照 ID>/`，再将原生路径替换为中央规则软链接；既有回滚快照仍会同时保留。指向未知位置的软链接、目录及其他不受支持的文件系统条目继续硬阻止，避免误接管外部数据。

### 范围、共享路径与模型服务

- 桌面控制台默认显示全局规则；输入一个已存在的绝对项目目录并点击“查看项目”，管理 Cursor、Copilot IDE、Continue、Junie 的项目规则。CLI 使用 `--project /absolute/project`，不会隐式采用当前工作目录或自动创建项目。
- Cursor 使用项目根 `AGENTS.md`，适用于官方支持该格式的版本；全局 User Rules 仍在 Cursor Settings 中管理。这个项目文件也可能被其他支持 AGENTS.md 的客户端读取。
- 默认 Gemini CLI 与 Antigravity 共用 `~/.gemini/GEMINI.md`，合并为一个开关；CLI 的 `gemini`、`antigravity`、`ag` 都可选择同一目标，不会重复写入，同一次请求中的相反操作会被拒绝。设置 `GEMINI_CLI_HOME` 后，Gemini 读取其下 `.gemini/GEMINI.md`，与 Antigravity 分开展示。
- 支持绝对路径环境覆盖：`CODEX_HOME`、`CLAUDE_CONFIG_DIR`、`COPILOT_HOME`、`QWEN_HOME`、`GEMINI_CLI_HOME`；OpenCode 按 `OPENCODE_CONFIG_DIR` → `XDG_CONFIG_HOME/opencode` → `~/.config/opencode` 选择。空值和相对路径不会被当成有效覆盖。
- Cline VS Code 与 Cline CLI 的全局目录分别适配；Linux 无 Documents 目录时使用 `~/Cline/Rules`。Kilo 项仅适配 legacy IDE extension；Zed 项仅适配原生 Agent；Junie 的自定义 Guidelines path 优先于默认路径。
- 规则保持完整，不截断。有明确单文件上限的客户端会在连接计划中阻止超长规则，并在状态中显示原因；切换 Profile 后已连接文件会立即跟随，若超限会在刷新状态时警告，需缩短 Profile。Factory、Augment 的限制还与其他规则共用预算，工具只检查本次输出。
- 项目链接依赖本机规则库，所有接入项目跟随本机当前 Profile；它不是每项目独立选 Profile 或可移植的团队文件导出。项目子目录中的软链接会阻止操作，回滚也检查目录边界与文件漂移。接入已有项目文件仍需明确确认备份接管。
- [Ollama](https://docs.ollama.com/quickstart) 和 [Z.ai / GLM](https://docs.z.ai/devpack/overview) 通过实际客户端接入。例如 Ollama 配合 Claude Code、Codex、OpenCode；Z.ai 配合 Claude Code、Cline、OpenCode。规则由客户端读取，不创建虚构的 provider 规则路径，不修改模型配置、API Key 或登录状态。
- 本目录是已核对原生入口的支持清单，不保证覆盖所有产品版本。需要启动参数、手动设置或特定插件才能读取规则的其他工具，不会被标记为自动接入。


## 操作历史与恢复

侧栏、Profile 页和控制台的「操作历史」入口统一展示本机规则库操作、全局及项目 Agent 接入，以及恢复操作。列表支持按类型筛选；时间使用本地时区，快照编号和完整路径在详情中查看。

- 「历史起点」表示本段可恢复历史开始前的状态，可用于撤销本段的第一条操作。选择普通记录表示恢复到**该操作完成之后**。预览会列出此后将撤销的全部操作、最终文件状态和内容对比；筛选不会缩小恢复范围。历史范围覆盖同一应用状态目录内的所有规则库和项目。
- 快照记录的是应用管理的文件变化，不是整个目录的完整版本。外部编辑器保存不会自动产生记录；受影响路径存在后续修改、父目录链接或项目边界异常时，整次恢复会在写入前被阻止。
- 确认绑定预览中的目标、历史链和文件状态。新增操作或文件变化使旧预览失效，需要重新检查。写入由本机状态目录中的互斥锁串行化。
- 恢复前保存反向快照，成功后以新记录追加到历史，保留原记录。要撤销本次恢复，选择它之前的最后一条操作并预览恢复。
- 旧版 CLI 回滚命令仍可用，但其归档快照没有完整的反向事件。此类回滚会让此前记录变为「仅供查看」；之后的新操作会建立新的可恢复历史链。
- 写入失败时尝试还原已完成的步骤，并拒绝覆盖期间发生的修改。若恢复被中断或无法完整补偿，保留状态目录的 `history/pending.json` 恢复材料并阻止后续修改。此时应先检查备份和实际文件状态；不要直接删除备份或强制覆盖。进程意外退出后遗留的 `mutation.lock` 也会阻止修改，需确认没有运行中的写入后再处理。

浏览器演示模式包含独立的示例历史，能够实际预览、恢复和撤销恢复，不读写真实规则目录。

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
cargo run -p arm-cli -- profiles delete work
cargo run -p arm-cli -- profiles delete work --apply
cargo run -p arm-cli -- profiles activate work
cargo run -p arm-cli -- profiles activate work --apply
cargo run -p arm-cli -- profiles rollback

# Agent 原生路径
cargo run -p arm-cli -- status
cargo run -p arm-cli -- plan
cargo run -p arm-cli -- apply --agents codex,claude
# 原生路径已有常规文件时，显式确认先备份再接管
cargo run -p arm-cli -- apply --agents codex --backup-existing
cargo run -p arm-cli -- plan --agents qwen --disconnect
cargo run -p arm-cli -- apply --agents qwen --disconnect
cargo run -p arm-cli -- rollback

# 项目规则：先预览，再应用（保留全局规则的同一套备份 / 回滚机制）
cargo run -p arm-cli -- --project /absolute/project status
cargo run -p arm-cli -- --project /absolute/project plan --agents cursor,copilot-ide
cargo run -p arm-cli -- --project /absolute/project apply --agents cursor,copilot-ide
cargo run -p arm-cli -- --project /absolute/project apply --agents cursor --disconnect
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
