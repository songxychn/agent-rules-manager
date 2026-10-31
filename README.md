# Agent Rules Manager

Agent Rules Manager 是一个 local-first 的桌面应用与 CLI：用多个 **Profile** 管理不同场景的规则，并把当前机器选中的 Profile 安全投射到 Codex、Claude Code、Cursor、Antigravity、Copilot 等 coding agent。

规则正文仍由用户在 VS Code、Cursor、Typora 等工具中维护；本项目负责结构校验、规则生成、本机切换、Agent 检测、原生路径接入、变更预览以及带漂移保护的回滚。

## 安装

从 [GitHub Releases](https://github.com/songxychn/agent-rules-manager/releases/latest) 下载当前版本的桌面安装包：

| 系统 | 安装包 |
| --- | --- |
| macOS Apple Silicon | `.dmg` |
| macOS Intel | `.dmg` |
| Windows | 当前用户 NSIS `.exe` |
| Linux | `.deb` 或 `.AppImage` |

这是 `0.x` 预发布。安装包方便直接打开图形界面。桌面端可在设置里检查更新并一键安装，不必再去 Releases 手动下载。Apple 公证和 Windows Authenticode 尚未配置。

- **macOS**：未公证的应用第一次打开时，按住 Control 点击图标，选择「打开」。
- **Windows**：接入 Agent 规则仍需要 [开发人员模式](https://learn.microsoft.com/windows/apps/get-started/enable-your-device-for-development)（或管理员权限）才能创建符号链接。
- **Linux**：`.deb` 适合 Debian/Ubuntu；其他发行版可用 AppImage。

试用问题请用 [Issue 模板](https://github.com/songxychn/agent-rules-manager/issues/new/choose) 提交，写明操作系统、版本、复现步骤和脱敏后的计划输出；敏感问题按 [安全策略](SECURITY.md) 私下报告。公开变更见 [更新日志](CHANGELOG.md)。维护者如何打 tag 打包见 [发布说明](docs/RELEASE.md)。

## 浏览器演示

不安装桌面端也可以先看流程。需要 Bun 1.4 或更高版本：

```bash
git clone https://github.com/songxychn/agent-rules-manager.git
cd agent-rules-manager
bun install --frozen-lockfile
bun run dev
```

打开 <http://127.0.0.1:1420>。浏览器模式使用纯内存演示数据，操作不会读取或修改真实规则库、Agent 配置，也不会启动本地编辑器；刷新页面会重置演示数据。按 `Ctrl+C` 停止服务。

要改本机配置，请用上面的安装包，或按 [桌面开发](#桌面开发) / [CLI](#cli) 从源码运行。桌面端目前以 macOS 为主要验证环境；Linux 与 Windows 安装包会随 Release 构建，但完整文件接入和回滚流程尚未建立发布验证矩阵，不承诺所有平台和 Agent 版本均已验证。

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
- 已有普通文件是合法的“独立使用”状态；接入前必须明确确认备份接管，绝不静默覆盖。
- 写入前必须生成计划；普通文件接管未经确认，或任一目标存在异常链接等硬冲突时，整次操作停止。
- 应用前持久化快照；回滚检测到应用后的人工修改时拒绝覆盖。

## 规则库布局

默认规则库位于 `~/.agent-rules`：

```text
~/.agent-rules/
├── schema.json
├── profiles/
│   ├── default/
│   │   ├── profile.json
│   │   └── AGENTS.md
│   └── work/
│       ├── profile.json
│       └── AGENTS.md
├── .runtime/                    # 本机生成，不同步
│   └── work-8d9f20b751a4/
│       ├── AGENTS.md
│       └── manifest.json
└── current -> .runtime/work-8d9f20b751a4/  # 本机选择，不同步
```

规则正文只属于 Profile；渲染结果属于 Runtime；`current` 只表达这台机器当前选择。格式细节见 [规则库格式](docs/RULE_LIBRARY.md)。

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
- 渲染结果带 `<!-- Source: profiles/<profile>/AGENTS.md -->` 注释，便于定位来源。

保存源文件不会自动更新已接入的 Agent。修改当前 Profile 的 `AGENTS.md` 后，需要预览并重新激活同一个 Profile，生成新的 Runtime；仅刷新界面或查询状态不会应用变更。所有已接入项目与全局目标都跟随本机当前 Profile，不支持每个项目独立选用 Profile。

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
2. 未经确认的普通文件接管、异常链接、重复 id 或非法路径阻止完整操作；确认接管仅适用于 Agent 原生路径中的普通文件。
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

客户端目录同时用于 Rust 适配器和浏览器演示，当前包含 25 个客户端入口（含旧版规则入口和 Windows 专用入口）。全局视图默认合并 Gemini CLI / Antigravity 的共享文件，macOS / Linux 显示 19 个目标，Windows 显示 20 个目标；项目视图显示 4 个目标。

| Agent | 范围 | 默认原生路径（未注明时为 macOS / Linux） |
| --- | --- | --- |
| [Claude Code](https://code.claude.com/docs/en/memory) | 全局 | `~/.claude/CLAUDE.md` |
| [Codex](https://learn.chatgpt.com/docs/agent-configuration/agents-md) | 全局 | `~/.codex/AGENTS.md` |
| [Grok Build](https://docs.x.ai/build/features/project-rules) | 全局 | `~/.grok/AGENTS.md` |
| [OpenCode](https://opencode.ai/docs/rules/) | 全局 | `~/.config/opencode/AGENTS.md` |
| [Qwen Code](https://qwenlm.github.io/qwen-code-docs/en/users/features/memory/) | 全局 | `~/.qwen/QWEN.md` |
| [Gemini CLI](https://geminicli.com/docs/cli/gemini-md/) | 全局 | `~/.gemini/GEMINI.md` |
| [Antigravity](https://antigravity.google/docs/rules) | 全局 | `~/.gemini/GEMINI.md` |
| [GitHub Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions) | 全局 | `~/.copilot/copilot-instructions.md` |
| [Devin Desktop（原 Windsurf）](https://docs.devin.ai/desktop/devin-local/customizations#rule-files) | 全局 | `~/.devin/global_rules.md` |
| [Windsurf 旧版（Cascade）](https://docs.devin.ai/desktop/cascade/memories) | 全局 | `~/.codeium/windsurf/memories/global_rules.md` |
| [Cline（当前版本）](https://docs.cline.bot/getting-started/config) | 全局 | `~/.cline/rules/agent-rules.md` |
| [Cline（Documents 规则目录）](https://docs.cline.bot/customization/cline-rules) | 全局 | `~/Documents/Cline/Rules/agent-rules.md` |
| [Cline CLI 旧版（独立规则目录）](https://docs.cline.bot/cli/cli-reference) | 全局 | `~/.cline/data/settings/rules/agent-rules.md` |
| [Roo Code（已停止维护）](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/docs/docs/features/custom-instructions.md) | 全局 | `~/.roo/rules/agent-rules.md` |
| [Kilo Code（旧版 VS Code 扩展）](https://github.com/Kilo-Org/kilocode-legacy/blob/main/docs/legacy-ides/customize/custom-rules.md) | 全局 | `~/.kilocode/rules/agent-rules.md` |
| [Kiro（IDE / CLI）](https://kiro.dev/docs/steering/) | 全局 | `~/.kiro/steering/agent-rules.md` |
| [Cursor](https://cursor.com/docs/context/rules) | 项目 | `<project>/AGENTS.md` |
| [GitHub Copilot (IDE)](https://docs.github.com/en/copilot/reference/custom-instructions-support) | 项目 | `<project>/.github/copilot-instructions.md` |
| [Continue](https://docs.continue.dev/customize/deep-dives/rules) | 项目 | `<project>/.continue/rules/agent-rules.md` |
| [Amp](https://ampcode.com/docs/customize/agents-md) | 全局 | `~/.config/amp/AGENTS.md` |
| [Factory Droid](https://docs.factory.com/harness/agents-md) | 全局 | `~/.factory/AGENTS.md` |
| [Augment Code / Auggie](https://docs.augmentcode.com/cli/rules) | 全局 | `~/.augment/rules/agent-rules.md` |
| [Zed Agent（macOS / Linux）](https://zed.dev/docs/ai/instructions) | 全局 | `~/.config/zed/AGENTS.md` |
| [Zed Agent（Windows）](https://zed.dev/docs/ai/instructions) | Windows 全局 | `%APPDATA%\Zed\AGENTS.md` |
| [Junie（JetBrains 插件）](https://junie.jetbrains.com/docs/junie-plugin-project-settings.html) | 项目 | `<project>/.junie/AGENTS.md` |

Adapter 只处理 Agent 协议差异；同步的 Profile 内容保持 CLI 和 Provider 中立。检测不会读取凭据、会话或登录状态。应用状态和备份位于平台应用数据目录，不放进规则库。

若 Agent 原生路径已有常规文件，接入计划不会再直接阻止启用，而会标记为“需确认接管”。确认后，程序先把完整旧文件保存到应用状态目录的 `backups/originals/<快照 ID>/`，再将原生路径替换为中央规则软链接；既有回滚快照仍会同时保留。指向未知位置的软链接、目录及其他不受支持的文件系统条目继续硬阻止，避免误接管外部数据。

### 范围、共享路径与模型服务

- 桌面控制台默认显示全局规则；输入一个已存在的绝对项目目录并点击“查看项目”，管理 Cursor、Copilot IDE、Continue、Junie 的项目规则。CLI 使用 `--project /absolute/project`，不会隐式采用当前工作目录或自动创建项目。
- Cursor 使用项目根 `AGENTS.md`，适用于官方支持该格式的版本；全局 User Rules 仍在 Cursor Settings 中管理。这个项目文件也可能被其他支持 AGENTS.md 的客户端读取。
- 默认 Gemini CLI 与 Antigravity 共用 `~/.gemini/GEMINI.md`，合并为一个开关；CLI 的 `gemini`、`antigravity`、`ag` 都可选择同一目标，不会重复写入，同一次请求中的相反操作会被拒绝。设置 `GEMINI_CLI_HOME` 后，Gemini 读取其下 `.gemini/GEMINI.md`，与 Antigravity 分开展示。
- Devin Desktop（原 Windsurf）使用 `~/.devin/global_rules.md`，由 Devin Local 读取，Devin CLI 也支持该路径；CLI 选择器为 `devin-desktop`（别名 `devin`）。检测兼容 Devin/Windsurf 应用、`devin-desktop`/`windsurf`/`surf` 命令及新旧配置目录；扩展检测兼容 `.devin/extensions` 和 `.windsurf/extensions`。
- 旧 `windsurf` ID 继续指向 Cascade 原有的 `~/.codeium/windsurf/memories/global_rules.md`，独立显示为“Windsurf 旧版（Cascade）”，保留已有连接、6,000 字符限制和回滚能力；不会自动改写、迁移或断开旧路径。启用 Devin Desktop 需要单独预览和确认；已有普通文件仍先确认接管并备份。
- [Devin 官方规则说明](https://docs.devin.ai/desktop/devin-local/customizations#rule-files)也支持项目 `AGENTS.md`、`.devin/rules/*.md` 和旧 `.windsurf/rules/*.md`、`.windsurfrules`；本入口只管理上述单个全局文件。新旧 `global_rules.md` 同时存在时，Devin Local 优先读取 `.devin/global_rules.md`，旧 `.windsurf/global_rules.md` 保留原样。其他规则文件和导入设置不会被本工具修改，是否导入旧格式取决于客户端配置。
- 支持绝对路径环境覆盖：`CODEX_HOME`、`CLAUDE_CONFIG_DIR`、`COPILOT_HOME`、`QWEN_HOME`、`GEMINI_CLI_HOME`、`CLINE_DIR`（Cline 当前版本）；OpenCode 按 `OPENCODE_CONFIG_DIR` → `XDG_CONFIG_HOME/opencode` → `~/.config/opencode` 选择。空值和相对路径不会被当成有效覆盖。
- Cline 当前版本的扩展、桌面版和 CLI 共用 `~/.cline/rules/agent-rules.md`（`cline-global`，别名 `cline-current`），支持 `CLINE_DIR` 覆盖配置根目录。旧 `cline` ID 继续管理 Documents 补充目录（Linux 无 Documents 目录时为 `~/Cline/Rules`）；旧 `cline-cli` ID 继续管理原 data/settings/rules 目录，已有连接不会自动迁移。当前客户端也会读取 Documents 目录，选择一个全局入口即可，避免重复加载。
- Kilo 项仅适配旧版 VS Code 扩展；当前 Kilo 通过 `kilo.jsonc` 的 `instructions` 配置加载文件，本工具不改写该配置。Zed 项仅适配原生 Agent。Junie 项明确针对 JetBrains 插件；自定义 Guidelines path 优先于默认路径，创建 `.junie/AGENTS.md` 也会改变其原有组合加载方式。Kiro 全局 Steering 用于本机 IDE / CLI，自定义 Agent 仍需在 `resources` 中引用。
- Zed 的 Windows 原生入口为 `zed-windows`，读取绝对路径 `APPDATA` 下的 `Zed/AGENTS.md`；`APPDATA` 缺失、为空或为相对路径时，使用用户主目录下的 `AppData/Roaming/Zed/AGENTS.md`。该入口只在 Windows 出现。旧 `zed` ID 继续对应 `.config/zed/AGENTS.md`，保留已有文件、连接和历史备份；Windows 上旧入口只按旧目录检测，不因检测到 Zed 命令或应用就被视为可用。接入新入口需单独预览，已有文件仍须确认备份接管；不会自动复制、覆盖或断开旧规则。macOS / Linux 继续使用原路径。
- 规则保持完整，不截断。有明确上限的客户端会按声明的字符数或 UTF-8 字节数在连接计划中阻止超长规则，并在状态中显示原因；切换 Profile 后已连接文件会立即跟随，若超限会在刷新状态时警告，需缩短 Profile。Antigravity 单文件上限为 24,000 字节（含展开引用），另有总 token 预算；Factory 的启动说明预算与其他文件共享。工具只检查本次渲染输出，无法计算客户端另行加载的文件和引用。Augment 的 IDE User Guidelines 与用户规则目录是不同入口，不把前者的 24,576 字符限制套到后者。
- 项目链接依赖本机规则库，所有接入项目跟随本机当前 Profile；它不是每项目独立选 Profile 或可移植的团队文件导出。项目子目录中的软链接会阻止操作，回滚也检查目录边界与文件漂移。接入已有项目文件仍需明确确认备份接管。
- [Ollama](https://docs.ollama.com/quickstart) 和 [Z.ai / GLM](https://docs.z.ai/devpack/overview) 通过实际客户端接入。例如 Ollama 配合 Claude Code、Codex、OpenCode；Z.ai 配合 Claude Code、Cline、OpenCode。规则由客户端读取，不创建虚构的 provider 规则路径，不修改模型配置、API Key 或登录状态。
- 本目录是已核对原生入口的支持清单，不保证覆盖所有产品版本。需要启动参数、手动设置或特定插件才能读取规则的其他工具，不会被标记为自动接入。


### 客户端核对（2026-10-31）

本轮逐项对照官方规则说明核对名称、路径和范围。验证覆盖本工具的预览、连接、备份与回滚，不等于所有客户端版本的实机加载验证。安装检测只识别应用、命令或配置标记，不确认客户端版本、登录状态或实际加载结果。

| 客户端 | 核对结果与处理 |
| --- | --- |
| [Claude Code](https://code.claude.com/docs/en/memory) | 保留用户 `CLAUDE.md` 路径；提示 Cowork 对工作区外全局软链接的限制。 |
| [Codex](https://learn.chatgpt.com/docs/agent-configuration/agents-md) | 保留默认路径及 `CODEX_HOME`；提示 `AGENTS.override.md` 优先，移除单独的 ChatGPT 应用检测标记。 |
| [Grok Build](https://docs.x.ai/build/features/project-rules) | 明确为官方编程 Agent，保留 `grok` ID，增加 `grok-build` 别名及命令检测；最新说明已取消旧版大小上限。 |
| [OpenCode](https://opencode.ai/docs/rules/)、[Qwen Code](https://qwenlm.github.io/qwen-code-docs/en/users/features/memory/)、[Gemini CLI](https://geminicli.com/docs/cli/gemini-md/) | 默认全局入口继续有效；保留现有环境覆盖。Gemini 自定义 `context.fileName` 时须自行确认文件名。 |
| [Antigravity](https://antigravity.google/docs/rules#size-limits-and-token-budgets) | 保留与 Gemini 共用路径；把旧 12,000 字符限制改为 24,000 UTF-8 字节，前后端区分单位。 |
| [Cline](https://docs.cline.bot/getting-started/config) | 增加当前版本统一全局规则目录；Documents 目录仍受支持；原 CLI 目录作为旧入口保留。 |
| [Roo Code](https://github.com/RooCodeInc/Roo-Code#disclaimer) | 标为已停止维护，保留原 ID 和规则路径；不把 Roomote 或 ZooCode 当成同一个适配器。 |
| [Kilo Code](https://kilo.ai/docs/customize/custom-rules) | 明确为旧版 VS Code 扩展；不声称现有目录会自动配置新版 CLI。 |
| [Kiro](https://kiro.dev/docs/steering/#steering-with-custom-agents) | 保留全局 Steering；注明自定义 Agent 需要资源引用，本机规则不自动作用于 Web / Mobile。 |
| [Cursor](https://cursor.com/docs/rules)、[Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions)、[Copilot IDE](https://docs.github.com/en/copilot/reference/custom-instructions-support) | 现有项目 / 全局范围正确；Cursor User Rules 仍由客户端设置管理，Copilot IDE 支持范围依客户端而定。 |
| [Continue](https://docs.continue.dev/customize/deep-dives/rules)、[Amp](https://ampcode.com/docs/customize/agents-md)、[Factory Droid](https://docs.factory.com/harness/agents-md) | 现有项目 / 全局文件入口继续有效；Continue 当前解析器支持无 frontmatter 的 Markdown；Factory 的 80,000 字符为共享启动预算。 |
| [Augment Code / Auggie](https://docs.augmentcode.com/cli/rules) | 保留用户规则目录；移除从 IDE User Guidelines 错套到用户规则的字符限制。 |
| [Zed Agent](https://zed.dev/docs/ai/instructions) | 已增加 Windows 专用原生路径及 APPDATA 回退；旧 ID、路径和连接独立保留。测试注入 Windows 平台并覆盖预览、接管备份、断开、回滚和漂移保护；尚未在真实 Windows 上进行端到端加载验证。外部 Agent 和终端线程保持各自的说明入口。 |
| [Junie](https://junie.jetbrains.com/docs/junie-plugin-project-settings.html) | 名称明确为 JetBrains 插件；保留项目默认路径，说明其对根规则、playbook / rules 和自定义路径的影响。 |

Continue 的纯 Markdown 处理另核对了官方仓库的 [markdownToRule 实现](https://github.com/continuedev/continue/blob/main/packages/config-yaml/src/markdown/markdownToRule.ts)，没有为此向中立规则源添加客户端专用 frontmatter。

## Git 同步（手动）

侧栏「Git 同步」可连接已有 Git 远端，把规则源在机器间共享。需要本机安装 Git；支持无内嵌凭据的 HTTPS、SSH 和绝对本地 Git 路径，认证使用系统 Git credential helper 或 SSH agent / 配置。SSH 使用非交互模式，首次认证、主机信任和密钥解锁需先在终端完成。本版不含自动同步、GitHub 登录或自动创建远端仓库。

1. 输入专用规则仓库地址和分支。首次默认 `master`；已有仓库请填写实际使用的分支。远端分支可为空，也可包含 v3 规则库。空的本机规则目录可以直接从远端导入。
2. 点击「检查并预览」，核对本机和远端各自将改变的文件及修改前后全文。预览只在独立临时 Git 仓库中检查远端，不改真实规则源、连接设置或激活状态。
3. 有冲突时对比基准、本机、远端，选择「保留本机」「使用远端」或「两个都保留」。保留两份需要给远端副本一个未占用的 Profile ID。删除与修改的冲突只提供前两种选择。选择后重新检查最终结果，所有冲突解决后才能应用。
4. 确认同步会先备份并应用本机规则源和连接状态，再以绑定预览版本的快进推送发布。任一端或本机选择变化会使预览失效；推送期间发生的并发更新、删除或回退也不会被覆盖。
5. **同步与激活分离。** 当前启用的 Profile 正文更新后，Runtime 标记为待刷新；到 Profile 页预览并激活后 Agent 才读取新规则。同步不改 `current`、`machine.json` 或 Agent 接入，也不会删除本机当前启用的 Profile；需先切换再同步删除。

传输严格限定为 `schema.json`、`profiles/<id>/profile.json` 和 `profiles/<id>/AGENTS.md`。不会提交规则库其他文件，也不操作规则库自身的 `.git` 或暂存区。远端已有普通 `.gitignore` 会原样保留；其他额外路径、链接、子模块、可执行源文件、无效格式均会阻止同步。首版支持 v3、最多 256 个 Profile、单文件 1 MiB、源文件合计 16 MiB，以及 SHA-1 Git 仓库。每个 Git 子命令超时为 45 秒。

首次连接或更换远端没有共同同步基准，同名但不同的 Profile 会明确列为冲突。已连接后按上次同步基准进行三方判断；不同 Profile 的修改可以组合，同一 Profile 的元数据字段与正文可独立组合，双方不同地修改同一正文时保留为冲突，不按时间选赢家。

推送失败会明确报告「本机规则源已保存，上传失败」，保留本机备份；重新检查后可重试。本机操作历史可以恢复同步造成的本机变化并检查漂移，**不会撤销已经完成的远端推送**；要发布恢复结果，需要再同步。连接地址与同步基准留在应用状态目录，不进入规则仓库。浏览器演示包含可操作的远端更新、冲突、同步和独立激活，刷新会重置。

CLI 示例（首次预览后保存输出中的 `token`；后续可省略已保存的地址与分支）：

```bash
cargo run -p arm-cli -- sync --remote git@github.com:you/agent-rules.git --branch master --json
# 核对以上预览，再传入其 token；有冲突时先按下文选择并重新预览
cargo run -p arm-cli -- sync --remote git@github.com:you/agent-rules.git --branch master --apply --token '<预览 token>' --json

# decisions.json 是冲突处理数组，例如：
# [{"id":"profile:work","choice":"keepBoth","newProfileId":"work-remote"}]
# choice 还可为 keepLocal / useRemote；schema 冲突的 id 为 schema
cargo run -p arm-cli -- sync --remote git@github.com:you/agent-rules.git --branch master --resolutions decisions.json --json
cargo run -p arm-cli -- sync --remote git@github.com:you/agent-rules.git --branch master --resolutions decisions.json --apply --token '<重新预览 token>' --json

# 同步后单独预览、激活
cargo run -p arm-cli -- profiles activate work
cargo run -p arm-cli -- profiles activate work --apply
```

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

初始化、创建、删除和激活 Profile 默认只输出计划，加 `--apply` 才执行。Agent 接入使用独立的 `plan` / `apply` 命令；两种 `rollback` 命令会直接尝试恢复最近快照，并检查漂移。以下命令默认面向本机真实规则库和 Agent 路径：

```bash
# 初始化、旧根文件迁移或 v1 / v2 → v3 升级
cargo run -p arm-cli -- init
cargo run -p arm-cli -- init --apply

# Profile
cargo run -p arm-cli -- profiles list
cargo run -p arm-cli -- profiles create work --name "Work"
cargo run -p arm-cli -- profiles create work --name "Work" --apply
cargo run -p arm-cli -- profiles delete work
cargo run -p arm-cli -- profiles delete work --apply
# 首次启用、切换，或编辑 AGENTS.md 后刷新同一 Profile
cargo run -p arm-cli -- profiles activate work
cargo run -p arm-cli -- profiles activate work --apply
cargo run -p arm-cli -- profiles rollback

# Agent 原生路径
cargo run -p arm-cli -- status
cargo run -p arm-cli -- plan --agents codex,claude
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

所有命令支持 `--root <directory>` 与 `--state-root <directory>`；查询和计划支持 `--json`。这两个参数只改变规则库和状态目录，不改变 Agent 原生目标路径，不能单独用作接入操作的隔离沙箱。

## 桌面开发

需要 Bun 1.4 或更高版本、Rust stable 及 [Tauri 2 平台依赖](https://v2.tauri.app/start/prerequisites/)。在仓库目录运行：

```bash
bun install --frozen-lockfile
bun run test
bun run build
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace
bun run tauri dev
# 本地生成当前平台安装包
bun run tauri:build
```

桌面端只允许用固定打开目标处理后端重新校验过的 Profile 文件；WebView 不能提交任意可执行文件或绝对路径。普通 `bun run dev` 使用纯内存演示数据，不读取或修改真实用户目录，也不会启动本地应用。

## 项目结构

```text
crates/arm-core/   Profile 校验、渲染、事务投射与回滚
crates/arm-cli/    CLI
src-tauri/         Tauri 命令与安全的外部打开边界
src/               React 控制台与惰性浏览器演示
docs/              数据格式、架构契约与发布流程
```

实现约束见 [架构说明](docs/ARCHITECTURE.md)，参与方式见 [贡献指南](CONTRIBUTING.md)，公开变更见 [更新日志](CHANGELOG.md)。

## 许可证

[MIT](LICENSE)
