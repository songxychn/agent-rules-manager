# Open target icons

These local SVG files identify applications or actions in the “Open AGENTS.md”
picker. No icons are extracted from installed applications, and no remote image
requests are made at runtime. SVG files are imported with `?no-inline` so they
remain local assets allowed by the desktop Content Security Policy.

## Sources and licenses

| Files | Upstream source | License provided by upstream |
| --- | --- | --- |
| `vscode.svg`, `intellij-idea.svg`, `rider.svg`, `webstorm.svg` | [Devicon v2.17.0](https://github.com/devicons/devicon/tree/v2.17.0) | [MIT](licenses/devicon-MIT.txt) |
| `cursor.svg` | [Lobe Icons](https://github.com/lobehub/lobe-icons/tree/a94750e3f5f8fc33757b839d85030e742284e43a) | [MIT](licenses/lobe-icons-MIT.txt) |
| `default.svg`, `typora.svg`, `textedit.svg`, `finder.svg` | [Microsoft Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons/tree/9cf8af0f95a555918a60b8147a2f33a6a1248442) | [MIT](licenses/fluentui-MIT.txt) |
| `terminal.svg` | [Icons8 Flat Color Icons](https://github.com/icons8/flat-color-icons/tree/1bf90d5ff118bc6690120ff9fdfe234565b7e414) | [MIT option of the upstream dual-license declaration](licenses/flat-color-icons.md) |

[provenance.json](provenance.json) records each exact download URL, retrieval date,
and SHA-256. SVG artwork is retained without recoloring or redrawing. Only `terminal.svg`
line endings are normalized from CRLF to LF; its upstream and local hashes are
both recorded. Other files retain the original bytes.
Only the selected files are vendored; the application does not depend on the
full icon packages. Preserve the accompanying license files when redistributing.

The Fluent icons depict Document, Document Edit, Text Edit Style, and Document
Folder. The Icons8 icon depicts Command Line. These are generic action symbols,
not official Typora, TextEdit, Finder, or Apple Terminal logos.

## Brand attribution

Visual Studio Code and its icon are trademarks of Microsoft Corporation.
IntelliJ IDEA, Rider, and WebStorm and their logos are trademarks or registered
trademarks of [JetBrains s.r.o.](https://www.jetbrains.com/).
Cursor and its mark belong to Anysphere, Inc. Other application names belong to
their respective owners. Use here identifies the application selected to open a
file; it does not imply affiliation, sponsorship, or endorsement.

The icon libraries' MIT licenses do not grant unrestricted trademark rights.
Brand assets remain subject to their owners' policies:

- [Visual Studio Code icon/name guidelines and Open in VS Code actions](https://code.visualstudio.com/brand).
- [JetBrains brand assets and integration guidelines](https://www.jetbrains.com/company/brand/).
- [Cursor brand guidelines](https://cursor.com/brand). This page provides brand
  resources but does not state a general redistribution license; the vendored
  drawing is sourced from Lobe Icons under its MIT license.

Do not use these marks as the Agent Rules Manager logo or imply vendor approval.
