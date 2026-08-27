# Rule Library format

The active format is intentionally small, JSON-based, and independent of any Agent CLI.

## Root schema

`schema.json` identifies Profile-owned schema v2 and its transport boundary:

```json
{
  "schemaVersion": 2,
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
├── AGENTS.md
└── rules/
    ├── safety.md
    └── review.md
```

```json
{
  "schemaVersion": 2,
  "id": "work",
  "name": "Work machine",
  "description": "Shared engineering and review conventions.",
  "instructions": [
    "AGENTS.md",
    "rules/safety.md",
    "rules/review.md"
  ]
}
```

Rules:

- `id`: 1–64 characters; begins with a lowercase ASCII letter or digit; remaining characters may also use `_` and `-`;
- `name`: non-empty, at most 80 characters;
- `description`: optional, at most 240 characters;
- `instructions`: non-empty and ordered;
- first instruction: exactly `AGENTS.md`;
- every instruction: a unique relative `.md` path with only normal path components;
- every source: a regular UTF-8 file; symlinks are rejected.

Creating a Profile writes `profile.json` and its required `AGENTS.md` together. Adding a supplemental source updates the manifest and creates the file in one previewed transaction. Existing paths are never overwritten. Removing a supplemental source updates the manifest and deletes that exact declared file in one rollback-protected transaction. The required `AGENTS.md` entrypoint cannot be removed, and undeclared paths are never touched.

Deleting a Profile is also previewed and rollback-protected. It is allowed only while the Profile is inactive on the current machine and only when its directory contains exactly `profile.json`, the declared instruction files, and their parent directories. Any undeclared file, directory, or symlink blocks the complete deletion. The transaction snapshots every owned file, removes the empty directory tree, and restores the files and directories on rollback unless a deleted path has since drifted.

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
    },
    {
      "path": "rules/review.md",
      "digest": "a1d19e80bc3e"
    }
  ]
}
```

The Profile digest includes logical source paths and exact contents. Renaming a source or changing content selects another Runtime directory; `current` changes only through a verified transaction.

## Schema v1 import

Schema v1 used `packs/**` plus `profiles/<id>.json`. It is accepted only for previewed migration to v2.

For every legacy Profile, migration copies referenced files in the old render order:

- the first source becomes `profiles/<profile>/AGENTS.md`;
- later files use deterministic paths under `profiles/<profile>/rules/<legacy-pack>/`;
- content is copied exactly before any old source is removed;
- unused legacy Packs and undeclared entries block migration to prevent loss;
- a rollback snapshot restores the v1 files and removes only newly created empty directories.

## Git baseline

For a dedicated rule-library repository, a minimal `.gitignore` is:

```gitignore
.runtime/
current
```

Application state and backups live outside the repository. Credentials, chat history, provider tokens, and Agent login state must never be added to the Profile library.
