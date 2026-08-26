# Contributing

Thanks for helping improve Agent Rules Manager.

## Development

1. Open an issue before making a behavior or safety-model change.
2. Keep the canonical rule format provider-neutral; add provider details to adapters.
3. Add a regression test for every filesystem edge case.
4. Run `cargo test --workspace`, `bun run test`, and `bun run build` before opening a pull request.

Changes that overwrite unmanaged files, apply without a previewable plan, or bypass rollback drift checks will not be accepted.

Use temporary home, rule-library, and state directories in tests. Tests must never write to the developer's real agent configuration paths.
