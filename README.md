# Agent Rules Manager

在同一套规则库里维护多组 Rule Pack，用 Profile 按顺序组合，并把当前机器选中的 Profile 安全投射到 Codex、Claude Code、Grok、OpenCode 等 coding agent。

Agent Rules Manager 是 local-first 的桌面应用与 CLI，面向同时使用 Codex、Claude Code、Grok、OpenCode、Qwen Code 等工具的人。规则正文继续由用户使用 VS Code、Cursor、Typora 等工具维护；本项目负责规则库校验、组合渲染、Profile 切换、已安装 Agent 检测、原生路径接入、变更预览和带漂移保护的回滚。

> 当前状态：未发布开发阶段。数据模型可以直接演进，不承担旧版本产品兼容；本机已有的 `~/.agent-rules/AGENTS.md` 会在创建精确回滚快照后迁入 base Rule Pack。

## 核心模型

- **Rule Pack**：一组可复用规则。每个 Pack 必须有 `AGENTS.md`，还可以在 `pack.json` 中按顺序声明其他 Markdown 文件。
- **Profile**：有序的 Rule Pack id 列表。顺序就是最终渲染顺序。
- **Runtime**：根据某个 Profile 生成的不可变 `AGENTS.md` 和来源清单，属于本机缓存。
- **`current`**：本机稳定入口，指向当前 Runtime。Agent 永远接入 `current/AGENTS.md`，切换 Profile 不需要重新接入。
- **Adapter**：把稳定入口映射到不同 Agent 的原生规则路径。

```text
Rule Pack: base ───┐
                   ├─ Profile: work ── render ── .runtime/work-<digest>/
Rule Pack: work ───┘                                  ▲
                                                     │
                                        current ─────┘
                                           │
                     ┌─────────────────────┼─────────────────────┐
                     v                     v                     v
                 Claude link           Codex link         OpenCode link
```

- 同时检查命令是否可用以及配置目录是否存在，避免桌面应用因 `PATH` 不完整漏掉已使用的 Agent。
- 接入的 Agent 使用指向 `current/AGENTS.md` 的符号链接；退出接入时把当前 Profile 规则落成普通文件，之后可独立修改。
- 已有普通文件属于合法的“独立使用”状态；选择接入时会被视为需人工处理的冲突，绝不静默覆盖。
- 写入前生成只读计划；任一选中目标存在异常链接或受保护的独立文件时，整次应用被阻止。
- 应用前持久化文件快照；回滚时若检测到应用后的人工修改，则拒绝覆盖。

## 规则库布局

默认规则库位于 `~/.agent-rules`：

```text
~/.agent-rules/
├── schema.json
├── packs/
│   ├── base/
│   │   ├── pack.json
│   │   ├── AGENTS.md
│   │   └── rules/
│   │       └── safety.md
│   └── work/
│       ├── pack.json
│       └── AGENTS.md
├── profiles/
│   ├── default.json
│   └── work.json
├── .runtime/                    # 本机生成，不同步
│   └── work-8d9f20b751a4/
│       ├── AGENTS.md
│       └── manifest.json
└── current -> .runtime/work-8d9f20b751a4/  # 本机选择，不同步
```

规则库根目录不再保留一个被所有 Agent 直接读取的 `AGENTS.md`。规则正文属于具体 Rule Pack；最终组合属于不可变 Runtime；`current` 只表达这台机器当前选择。这样多套规则可以共存，切换时也不会改写任何源文件。

格式细节和示例见 [Rule Library](docs/RULE_LIBRARY.md)。

## 多机器同步边界

当前实现已经固定传输层契约，但尚未内置 Git push/pull：

| 参与同步 | 只保留在本机 |
| --- | --- |
| `schema.json` | `.runtime/**` |
| `packs/**` | `current` |
| `profiles/**` | 应用状态目录中的 `machine.json` |
| 用户选择纳入版本控制的普通说明文件 | 投射/规则库回滚快照 |
|  | Agent 原生路径和登录凭据 |

因此不同机器可以同步同一批 Rule Pack 和 Profile，但分别启用 `work`、`personal` 或其他 Profile。规则库初始化时会在不存在 `.gitignore` 的情况下创建 `.runtime/` 与 `current` 忽略项；如果用户已有 `.gitignore`，程序不会覆盖它。

后续 Git 传输层只允许处理同步列中的路径，并采用显式 pull/merge/push、冲突可见、凭据交给系统 Git/SSH 的方式。它不会把“当前启用哪个 Profile”当成跨机器状态。

## Rule Pack 示例

`packs/base/pack.json`：

```json
{
  "schemaVersion": 1,
  "id": "base",
  "name": "Base rules",
  "description": "Rules shared by every local profile.",
  "instructions": [
    "AGENTS.md",
    "rules/safety.md",
    "rules/review.md"
  ]
}
```

约束：

- `AGENTS.md` 必须存在且排在第一位；
- 补充文件必须是 Pack 目录内的相对 `.md` 路径；
- 不允许绝对路径、`..`、重复路径或符号链接源；
- 渲染结果带 `<!-- Source: packs/<pack>/<file> -->` 注释，便于定位来源。

## 安全模型

所有文件系统写入遵循同一套边界：

1. 先生成可读计划，不写文件。
2. 未托管文件、异常链接、重复 id 或非法路径会阻止整次操作。
3. 应用前把每个将改变的目标及其原始状态持久化到本机状态目录。
4. 应用阶段再次校验原始状态；中途失败会只恢复仍处于预期状态的目标。
5. 回滚遇到应用后的人工修改会停止，不覆盖漂移。
6. Runtime 是按摘要命名的不可变生成缓存；已存在但内容不符时视为冲突。

创建新 Pack、追加 Markdown、创建 Profile、切换 Profile、接入 Agent 都需要“预览 → 确认”两步。浏览器演示模式只修改内存数据，不读取或改写真实用户目录。

### 旧根文件迁移

当没有 `schema.json` 且发现普通文件 `~/.agent-rules/AGENTS.md` 时，初始化计划会：

1. 在本机应用状态目录中备份旧文件的精确内容和状态；
2. 把内容复制到 `packs/base/AGENTS.md`；
3. 创建默认 Profile 与 Runtime；
4. 删除规则库根目录的旧 `AGENTS.md`，不再保留第二个规则入口；
5. 把仍指向旧路径的已托管 Agent 接入标为待刷新。

迁移与删除会出现在同一份预览和回滚快照中；复制失败或应用时发现漂移都会中止，显式回滚可以恢复旧根文件。

## Agent 接入

| Agent | 原生路径 | 接入方式 |
| --- | --- | --- |
| Claude Code | `~/.claude/CLAUDE.md` | 可选符号链接 / 独立文件 |
| Codex | `~/.codex/AGENTS.md` | 可选符号链接 / 独立文件 |
| Grok | `~/.grok/AGENTS.md` | 可选符号链接 / 独立文件 |
| OpenCode | `$OPENCODE_CONFIG_DIR/AGENTS.md`、`$XDG_CONFIG_HOME/opencode/AGENTS.md` 或 `~/.config/opencode/AGENTS.md` | 可选符号链接 / 独立文件 |
| Qwen Code | `$QWEN_HOME/QWEN.md` 或 `~/.qwen/QWEN.md` | 可选符号链接 / 独立文件 |

默认 adapter 目录是受支持 Agent 的检测目录。检测结果只决定界面是否提供接入开关，不读取凭据、会话或登录状态。旧版 Claude 托管 include 会被识别为“旧版接入”；退出时会把中央规则复制进原文件并保留 include 周围的 Claude 专属内容。

应用自身的投射快照和状态不放在规则库里。macOS 使用 `~/Library/Application Support/agent-rules-manager`，Linux 使用 `$XDG_STATE_HOME/agent-rules-manager` 或 `~/.local/state/agent-rules-manager`。

Adapter 只处理 Agent 协议差异；Rule Pack 内容保持 CLI/Provider 中立。

## CLI

CLI 的规则库写命令默认只输出计划，加 `--apply` 才确认执行：

```bash
# 初始化或带回滚快照的迁移
cargo run -p arm-cli -- init
cargo run -p arm-cli -- init --apply

# Rule Pack
cargo run -p arm-cli -- packs list
cargo run -p arm-cli -- packs create team --name "Team rules"
cargo run -p arm-cli -- packs create team --name "Team rules" --apply
cargo run -p arm-cli -- packs add-file team rules/review.md
cargo run -p arm-cli -- packs add-file team rules/review.md --apply

# Profile（Pack 参数顺序即渲染顺序）
cargo run -p arm-cli -- profiles create work --name "Work" --packs base,team
cargo run -p arm-cli -- profiles create work --name "Work" --packs base,team --apply
cargo run -p arm-cli -- profiles activate work
cargo run -p arm-cli -- profiles activate work --apply

# Agent 原生路径
cargo run -p arm-cli -- status
cargo run -p arm-cli -- plan
cargo run -p arm-cli -- apply --agents codex,claude
cargo run -p arm-cli -- plan --agents qwen --disconnect
cargo run -p arm-cli -- apply --agents qwen --disconnect
cargo run -p arm-cli -- rollback
```

所有命令支持 `--root <directory>` 和 `--state-root <directory>`；查询与计划类命令支持对应的 `--json`。

## 桌面开发

需要 Bun 1.4+、Rust stable 以及 [Tauri 2 平台依赖](https://v2.tauri.app/start/prerequisites/)：

```bash
bun install --frozen-lockfile
bun run test
bun run build
cargo test --workspace
bun run tauri dev
```

桌面控制台会检测当前系统中可用的常见编辑器，并通过右上角“上次选择 + 下拉切换”的紧凑分体按钮打开当前 Pack 的 `AGENTS.md`。Finder / 文件资源管理器用于显示文件，终端目标打开规则库目录；前端只提交固定目标 id 和已校验的 Pack 文件标识，实际路径和启动参数由 Tauri 后端决定。

普通 `bun run dev` 会运行浏览器演示数据，不读取或修改真实用户目录，也不会启动本地应用；只有 Tauri 后端会执行文件检查和外部打开操作。

## 项目结构

```text
crates/arm-core/   规则库校验、组合渲染、事务投射与回滚
crates/arm-cli/    CLI
src-tauri/         Tauri 命令与安全的外部打开边界
src/               React 控制台和惰性浏览器演示
docs/              数据格式与架构契约
```

路线图包括显式 Git 多机器传输、可配置 adapters、受控导入现有独立规则和签名发布。实现约束见 [架构说明](docs/ARCHITECTURE.md)，参与方式见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## License

[MIT](LICENSE)
