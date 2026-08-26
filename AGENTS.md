# Repository rules

Agent Rules Manager is a local-first configuration tool. Treat user instruction files as valuable data.

## Safety invariants

- Never overwrite an unmanaged regular file.
- Preview every filesystem mutation before apply.
- Back up every changed target and make rollback refuse to overwrite post-apply drift.
- Keep the canonical rules source CLI-neutral; agent-specific behavior belongs in adapters.
- Do not place credentials, chat history, or provider authentication in the managed rule library.

## Verification

- Run `cargo test --workspace` after Rust changes.
- Run `bun run test` and `bun run build` after frontend changes.
- Keep browser fallback data functional so UI work does not require mutating real home-directory files.
