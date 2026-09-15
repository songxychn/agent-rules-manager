# Desktop releases

Push a version tag to build installers and attach them to a GitHub Release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The [Release](../.github/workflows/release.yml) workflow produces:

| Platform | Artifact |
| --- | --- |
| macOS Apple Silicon | `.dmg` plus updater `.app.tar.gz` |
| macOS Intel | `.dmg` plus updater `.app.tar.gz` |
| Windows | current-user NSIS `.exe` |
| Linux | `.deb`, `.AppImage`, and updater signature |

It also uploads `latest.json` so installed apps can check GitHub Releases and install the update from Settings.

`workflow_dispatch` rebuilds the same artifacts against the current version string as a draft.

## Updater signing

In-app updates require a minisign key pair. The public key is in `src-tauri/tauri.conf.json`. The private key lives in gitignored `.tauri/updater.key` on the machine that generated it.

Set these repository secrets before the first tagged release:

- `TAURI_SIGNING_PRIVATE_KEY` — contents of `.tauri/updater.key`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — empty unless you created the key with a password

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY < .tauri/updater.key
```

If this key is lost, already-installed apps cannot verify future updates until users install a new build that embeds a replacement public key.

The updater endpoint is `https://github.com/songxychn/agent-rules-manager/releases/latest/download/latest.json`. GitHub only serves that URL for a public, non-prerelease latest release.

Local packages that create updater artifacts also need the private key in the environment:

```bash
export TAURI_SIGNING_PRIVATE_KEY_PATH="$PWD/.tauri/updater.key"
bun run tauri:build
```

## Apple and Windows OS signing

Unsigned packages still install. Ordinary macOS users will see a Gatekeeper warning until Apple Developer ID signing and notarization are configured. Ordinary Windows users may see SmartScreen until Authenticode signing is added.

Optional repository secrets for macOS notarization:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

See [Tauri macOS signing](https://v2.tauri.app/distribute/sign-macos/).
