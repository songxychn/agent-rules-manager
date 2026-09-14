# Contributing

Thanks for helping improve Agent Rules Manager.

This is an early `0.x` source release for developers. See the [README](README.md#开发者试用) for browser-demo and desktop setup. The browser demo uses in-memory data and is the preferred starting point for UI work.

## Development

1. Open an issue before making a behavior or safety-model change.
2. Keep Profile instruction content provider-neutral; add provider details to adapters.
3. Add a regression test for every filesystem edge case.
4. Run `cargo test --workspace`, `bun run test`, and `bun run build` before opening a pull request.

Changes that silently overwrite unmanaged files, apply without a previewable plan, sync machine-local state, or bypass rollback drift checks will not be accepted. Taking over an existing regular Agent file requires explicit confirmation and a complete readable backup before replacement; foreign symlinks and unsupported entries remain hard conflicts.

Use temporary home, rule-library, and state directories in tests. Tests must never write to the developer's real agent configuration paths.

For bug reports, include the operating system, relevant tool versions, reproduction steps, and sanitized plan output. Do not attach your real rules library, credentials, or home-directory contents. Follow [SECURITY.md](SECURITY.md) for vulnerability reports.
