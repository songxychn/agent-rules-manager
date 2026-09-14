# Rule Library format

The active format is intentionally small, JSON-based, and independent of any Agent CLI.

## Root schema

`schema.json` identifies single-file Profile schema v3 and its transport boundary:

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

The `sync` object declares policy for humans and a future transport. It does not grant permission to scan or mutate arbitrary matching paths.

A library created from the former root `AGENTS.md` also records `"legacySource": "AGENTS.md"`. This provenance is used only to recognize native Agent links that still point at the removed path. No other value is accepted, and the root file does not remain a source.

## Profile

Directory name and `id` must match:

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

Rules:

- `id`: 1–64 characters; begins with a lowercase ASCII letter or digit; remaining characters may also use `_` and `-`;
- `name`: non-empty, at most 80 characters;
- `description`: optional, at most 240 characters;
- the only rule source is `AGENTS.md`, a regular UTF-8 file; symlinks are rejected;
- v3 metadata has no configurable source list. Supplemental sources are not supported.

Creating a Profile writes `profile.json` and `AGENTS.md` together in a previewed transaction. Existing paths are never overwritten. Edit all rules in that AGENTS.md; no add-file or remove-file command is exposed.

Deleting an inactive Profile snapshots its metadata and AGENTS.md, then removes its directory. Any undeclared file, directory, or symlink blocks deletion. Rollback restores the original files unless a target has since drifted.

Profile documents contain no active flag. The same synced Profile can be active on one machine and inactive on another without changing source files.

## Machine selection

The application-state directory, not the rule library, contains:

```json
{
  "schemaVersion": 1,
  "activeProfileId": "work"
}
```

The matching library entry is a local relative symlink:

```text
current -> .runtime/work-8d9f20b751a4
```

| Machine | Available Profiles | Local `activeProfileId` |
| --- | --- | --- |
| company-mac | `default`, `work`, `personal` | `work` |
| home-mac | `default`, `work`, `personal` | `personal` |

Neither selection modifies a Profile manifest.

## Runtime manifest

Runtime output is generated and must not be hand-edited:

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

The Profile digest includes logical source paths and exact contents. Changing AGENTS.md content selects another Runtime directory; `current` changes only through a verified transaction.

## Schema v1 / v2 import

Both legacy formats require a previewed upgrade to v3. V1 used `packs/**` and `profiles/<id>.json`; v2 stored an ordered `instructions` list in each Profile manifest.

Migration concatenates each Profile's source contents in the original order into its AGENTS.md, adding only separating newlines. A single source is preserved byte for byte. Original files and metadata are snapshotted before writes; supplemental files are removed only after their merged content is prepared. V2 supplemental directories are removed once empty, except ancestors of undeclared empty directories. Undeclared empty directory trees are preserved and do not block v2 migration. Undeclared files, symlinks, and unused v1 Packs still block migration.

The local active Profile is preserved and regenerated. A library without a local selection remains unselected. Source migration and runtime activation use separate existing snapshots; restore activation first, then migration. Rollback refuses to overwrite post-apply drift.

## Git baseline

For a dedicated rule-library repository, a minimal `.gitignore` is:

```gitignore
.runtime/
current
```

Application state and backups live outside the repository. Credentials, chat history, provider tokens, and Agent login state must never be added to the Profile library.
