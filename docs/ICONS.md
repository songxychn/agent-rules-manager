# macOS 图标

`assets/icon-source.svg` 是应用内使用的品牌标识，背景铺满画布。Dock 图标在生成时将整个标识居中缩放至约 80.5%：在 1024 × 1024 画布中，主体为 824 × 824 像素，四周各保留 100 像素透明区域。实际占比为 80.46875%，按整数像素对齐，与本机已检查的 Excel、VS Code、Chrome 传统图标主体比例一致，避免底板比相邻应用显得更大。

在安装依赖后，指定一个尚不存在的输出目录：

```bash
bun tools/generate-macos-icons.ts /tmp/agent-rules-manager-macos-icons
```

脚本从品牌源文件生成带留白的 SVG，再调用 Tauri 图标生成器。输出目录必须不存在；脚本不会覆盖仓库图标。先检查输出目录中的 `icons/icon.png`，备份仓库中的对应文件，再将以下文件从输出的 `icons/` 复制到 `src-tauri/icons/`：

- `icon.icns`：macOS 打包图标。
- `32x32.png`、`64x64.png`、`128x128.png`、`128x128@2x.png`、`icon.png`：开发运行及共享桌面 PNG。

Tauri 还会生成 ICO、Windows 商店和移动端资源，本次 macOS 流程不复制这些文件。应用内品牌标识也保持原样。重新启动开发进程或重新构建 `.app` 后，应用才能使用新图标；已有安装包不会自动更新。
