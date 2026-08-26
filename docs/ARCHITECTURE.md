# Architecture

## Boundaries

Agent Rules Manager separates user-owned content from application-owned state.

- **Rule library**: a directory selected by the user, containing the canonical `AGENTS.md`. It can be versioned with Git, Syncthing, chezmoi, or any other tool.
- **Adapters**: small definitions that map the canonical source into each agent's native rule path and deployment mode.
- **Application state**: pre-apply projection snapshots and local interface preferences. This is machine-local and must not be stored beside the user's rules by default.
- **Surfaces**: `agent-rules` CLI and the Tauri application call the same Rust core. The React browser mode is explicitly inert demo data.

```text
                  inspect / plan / apply
React + Tauri  ───────────────────────────┐
                                         v
CLI ───────────────────────────────> arm-core
                                         │
           ┌─────────────────────────────┼─────────────────────────────┐
           v                             v                             v
 canonical AGENTS.md             native agent paths             local snapshots
 (user-owned content)          (adapter projections)           (app-owned state)
```

## External editing boundary

The canonical `AGENTS.md` remains user-owned and is not edited inside the application. Workspace snapshots expose only its path, content digest, and modification time to the WebView. Users open the file in an installed editor, reveal it in the platform file manager, or open its parent directory in a terminal.

The frontend can submit only an open-target id returned by the Tauri backend. The backend re-resolves that id, derives the canonical source path from the active library, verifies that it is a regular file, and launches a fixed platform adapter. It never accepts an executable name, shell command, or arbitrary file path from the WebView. Browser demo mode returns representative targets but never launches a local application.

External source edits update symlink and managed-include projections through the stable source path; they do not require another apply. Projection rollback restores only native agent targets and never overwrites the canonical source.

## Adapter contract

An adapter currently declares an id, display label, target path, and deployment mode:

- `symlink`: valid only when the target is absent or already links to the canonical source. A regular file and a link to another location are conflicts.
- `include`: preserves an existing regular file and owns only the text between the Agent Rules Manager markers. Missing or malformed markers are never guessed at.

New adapter modes should implement three operations as one coherent contract:

1. inspect the target into `inSync`, `ready`, `drifted`, or `conflict`;
2. derive the exact desired file state without writing;
3. apply that derived state in a way rollback can verify.

Agent-specific content must not leak into the canonical source. If a provider needs unique instructions, its adapter should preserve them outside the managed projection or generate a thin provider-owned wrapper.

## Transaction model

Apply is optimistic but fail-closed:

1. resolve and validate the selected adapter ids;
2. inspect all targets and reject the complete plan on any conflict;
3. derive the desired state and its digest for every changed target;
4. persist all original states plus expected digests;
5. apply each target; automatically restore originals after an in-process error;
6. retain the snapshot for an explicit rollback.

Rollback accepts a target only when its digest matches either the expected applied state or its exact original state. The second case makes a partially completed process crash recoverable. Any other content is post-apply drift and blocks the complete rollback.

## Near-term extension points

- configuration file for custom agents and target paths;
- explicit import flow for existing unmanaged rule files;
- optional custom editor registration with the same fixed-id command boundary;
- Windows junction/copy strategy with platform-specific guarantees;
- signed desktop releases and updater metadata.
