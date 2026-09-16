# 规则库格式

现行格式刻意保持精简，基于 JSON，不依赖任何 Agent CLI。

## 根 schema

`schema.json` 标识单文件 Profile schema v3 及其传输边界：

```json
{
  "schemaVersion": 3,
  "format": "agent-rules-library",
  "sync": {
    "include": [
      "schema.json",
      "profiles/**"
    ],
    "exclude": [
      ".runtime/**",
      "current",
      "machine state"
    ],
    "machineSelection": "local"
  }
}
```

`sync` 对象向人和未来传输层声明策略。它并不授权扫描或改写任意匹配路径。

由旧根文件 `AGENTS.md` 创建的规则库还会记录 `"legacySource": "AGENTS.md"`。这条来源信息只用于识别仍指向已删除路径的 Agent 原生链接。不接受其他取值，根文件也不再作为源。

## Profile

目录名必须与 `id` 一致：

```text
profiles/work/
├── profile.json
└── AGENTS.md
```

```json
{
  "schemaVersion": 3,
  "id": "work",
  "name": "Work machine",
  "description": "Shared engineering and review conventions."
}
```

规则：

- `id`：1–64 个字符；以小写 ASCII 字母或数字开头；其余字符还可使用 `_` 和 `-`；
- `name`：非空，最多 80 个字符；
- `description`：可选，最多 240 个字符；
- 唯一规则源是普通 UTF-8 文件 `AGENTS.md`；符号链接会被拒绝；
- v3 元数据没有可配置的源文件列表，不支持补充源。

创建 Profile 会在带预览的事务中同时写入 `profile.json` 和 `AGENTS.md`。已有路径绝不覆盖。所有规则都在该 AGENTS.md 中编辑；不提供增删文件命令。

删除未启用的 Profile 会先快照其元数据和 AGENTS.md，再删除目录。任何未声明的文件、目录或符号链接都会阻止删除。除非目标之后已漂移，回滚会恢复原文件。

Profile 文档不含启用标记。同一套已同步 Profile 可以在一台机器上启用、在另一台机器上停用，而不改源文件。

## 本机选择

应用状态目录（不是规则库）包含：

```json
{
  "schemaVersion": 1,
  "activeProfileId": "work"
}
```

规则库里对应的是本机相对符号链接：

```text
current -> .runtime/work-8d9f20b751a4
```

| 机器 | 可用 Profile | 本机 `activeProfileId` |
| --- | --- | --- |
| company-mac | `default`、`work`、`personal` | `work` |
| home-mac | `default`、`work`、`personal` | `personal` |

选择不会改写 Profile 清单。

该机器上所有全局和项目 Agent 链接都跟随这一选择。项目接入不会单独选择 Profile，也不是给其他机器用的可移植导出。

## Runtime 清单

Runtime 输出由程序生成，不要手工编辑：

```json
{
  "schemaVersion": 2,
  "profileId": "work",
  "profileDigest": "8d9f20b751a4",
  "renderedDigest": "f12c8a32f10b",
  "sources": [
    {
      "path": "AGENTS.md",
      "digest": "62a4fb4f20c1"
    }
  ]
}
```

Profile 摘要包含逻辑源路径和精确内容。改 AGENTS.md 内容会选用另一个 Runtime 目录；`current` 只通过已校验的事务变更。

编辑源文件会使当前 Runtime 变为过期，但不会自动重新生成。再次预览并启用同一 Profile，才会刷新已接入的 Agent。读取状态或刷新界面不会应用源变更。

## Schema v1 / v2 导入

两种旧格式都需要带预览的升级到 v3。v1 使用 `packs/**` 和 `profiles/<id>.json`；v2 在每个 Profile 清单里保存有序的 `instructions` 列表。

迁移按原顺序把每个 Profile 的源内容拼进其 AGENTS.md，只补充分隔换行。单一源会逐字节保留。写入前先快照原文件和元数据；补充文件只在合并内容准备好后移除。v2 的补充目录在变空后删除，但未声明空目录的祖先除外。未声明的空目录树会保留，不阻止 v2 迁移。未声明文件、符号链接和未使用的 v1 Pack 仍会阻止迁移。

本机当前 Profile 会保留并重新生成。没有本机选择的规则库保持未选择。源迁移与 Runtime 启用使用各自已有的快照；先恢复启用，再恢复迁移。回滚拒绝覆盖应用后的漂移。

## Git 基线

若规则库单独成仓，最小 `.gitignore` 为：

```gitignore
.runtime/
current
```

应用状态和备份放在仓库外。凭据、聊天记录、Provider 令牌和 Agent 登录状态绝不能加入 Profile 库。
