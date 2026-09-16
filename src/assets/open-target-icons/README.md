# 打开目标图标

这些本地 SVG 用在「打开 AGENTS.md」选择器里，标识应用程序或操作。不会从已安装应用提取图标，运行时也不会请求远程图片。SVG 以 `?no-inline` 导入，从而作为桌面内容安全策略允许的本地资源保留。

## 来源与许可证

| 文件 | 上游来源 | 上游提供的许可证 |
| --- | --- | --- |
| `vscode.svg`、`intellij-idea.svg`、`rider.svg`、`webstorm.svg` | [Devicon v2.17.0](https://github.com/devicons/devicon/tree/v2.17.0) | [MIT](licenses/devicon-MIT.txt) |
| `cursor.svg` | [Lobe Icons](https://github.com/lobehub/lobe-icons/tree/a94750e3f5f8fc33757b839d85030e742284e43a) | [MIT](licenses/lobe-icons-MIT.txt) |
| `default.svg`、`typora.svg`、`textedit.svg`、`finder.svg` | [Microsoft Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons/tree/9cf8af0f95a555918a60b8147a2f33a6a1248442) | [MIT](licenses/fluentui-MIT.txt) |
| `terminal.svg` | [Icons8 Flat Color Icons](https://github.com/icons8/flat-color-icons/tree/1bf90d5ff118bc6690120ff9fdfe234565b7e414) | [上游双许可证声明中的 MIT 选项](licenses/flat-color-icons.md) |

[provenance.json](provenance.json) 记录每个文件的精确下载地址、获取日期和 SHA-256。SVG 图样保留原样，不重新着色或重绘。仅 `terminal.svg` 的换行从 CRLF 规范为 LF；上游哈希与本地哈希均有记录。其余文件保留原始字节。
只内嵌所选文件；应用不依赖完整图标包。再分发时请一并保留随附的许可证文件。

Fluent 图标表示文档、文档编辑、文本编辑样式和文档文件夹。Icons8 图标表示命令行。这些是通用操作符号，不是 Typora、TextEdit、Finder 或 Apple Terminal 的官方标志。

## 品牌归属

Visual Studio Code 及其图标是 Microsoft Corporation 的商标。
IntelliJ IDEA、Rider、WebStorm 及其标志是 [JetBrains s.r.o.](https://www.jetbrains.com/) 的商标或注册商标。
Cursor 及其标识属于 Anysphere, Inc.。其他应用名称归各自所有者。此处使用仅用于标识打开文件时所选应用，不表示存在关联、赞助或背书。

图标库的 MIT 许可证并不授予不受限制的商标权。品牌资产仍受所有者政策约束：

- [Visual Studio Code 图标/名称规范及 Open in VS Code 操作](https://code.visualstudio.com/brand)
- [JetBrains 品牌资产与集成规范](https://www.jetbrains.com/company/brand/)
- [Cursor 品牌规范](https://cursor.com/brand)。该页提供品牌资源，但未声明通用再分发许可；内嵌图样来自 Lobe Icons，按其 MIT 许可证使用。

不要把这些标志当作 Agent Rules Manager 的 logo，也不要暗示得到厂商认可。
