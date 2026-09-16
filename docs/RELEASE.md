# 桌面发布

推送版本 tag 会构建安装包并挂到 GitHub Release：

```bash
git tag v0.1.0
git push origin v0.1.0
```

[Release](../.github/workflows/release.yml) 工作流会产出：

| 平台 | 产物 |
| --- | --- |
| macOS Apple Silicon | `.dmg` 以及更新器用的 `.app.tar.gz` |
| macOS Intel | `.dmg` 以及更新器用的 `.app.tar.gz` |
| Windows | 当前用户 NSIS `.exe` |
| Linux | `.deb`、`.AppImage` 以及更新器签名 |

同时会上传 `latest.json`，已安装的应用即可在 GitHub Releases 检查更新，并从设置里安装。

`workflow_dispatch` 会按当前版本号重建同样产物，并保存为草稿 Release。

## 更新器签名

应用内更新需要一对 minisign 密钥。公钥写在 `src-tauri/tauri.conf.json`。私钥放在生成它的那台机器上、已被 gitignore 的 `.tauri/updater.key`。

首次打 tag 发布前，请配置这些仓库 secret：

- `TAURI_SIGNING_PRIVATE_KEY` — `.tauri/updater.key` 的内容
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — 若生成密钥时未设密码则留空

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < .tauri/updater.key
```

密钥丢失后，已安装的应用无法校验后续更新，除非用户改装嵌入了新公钥的构建。

更新器地址是 `https://github.com/songxychn/agent-rules-manager/releases/latest/download/latest.json`。GitHub 只为公开仓库上的非预发布最新 Release 提供该 URL。

本地打包若生成更新器产物，也需要在环境中提供私钥：

```bash
export TAURI_SIGNING_PRIVATE_KEY_PATH="$PWD/.tauri/updater.key"
bun run tauri:build
```

## Apple 与 Windows 系统签名

未签名的安装包仍可安装。在配置 Apple Developer ID 签名和公证之前，普通 macOS 用户会看到 Gatekeeper 警告。在加入 Authenticode 之前，普通 Windows 用户可能看到 SmartScreen。

未配置这些 secret 时不要把空的 `APPLE_*` 环境变量传给打包步骤，否则 Tauri 会尝试导入证书并失败。配置证书后，流水线会自动启用签名和公证。

macOS 公证可选的仓库 secret：

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

说明见 [Tauri macOS 签名](https://v2.tauri.app/distribute/sign-macos/)。
