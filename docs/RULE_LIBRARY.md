# Rule Library format

The format is intentionally small, JSON-based, and independent of any Agent CLI.

## Root schema

`schema.json` identifies the format and records the transport boundary:

```json
{
  "schemaVersion": 1,
  "format": "agent-rules-library",
  "sync": {
    "include": [
      "schema.json",
      "packs/**",
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

The `sync` field is a policy declaration for humans and a future transport; local library validation does not scan arbitrary files from it.

A library created by migrating the former root `AGENTS.md` also records
`"legacySource": "AGENTS.md"`. This is migration provenance used only to recognize and refresh native Agent links that still point at the removed path. It does not make the root file a source or stable entrypoint; no other value is accepted.

## Rule Pack

Directory name and `id` must match:

```text
packs/base/
├── pack.json
├── AGENTS.md
└── rules/
    ├── safety.md
    └── review.md
```

```json
{
  "schemaVersion": 1,
  "id": "base",
  "name": "Base rules",
  "description": "Shared engineering defaults.",
  "instructions": [
    "AGENTS.md",
    "rules/safety.md",
    "rules/review.md"
  ]
}
```

Rules:

- id: 1–64 characters; begins with a lowercase ASCII letter or digit; remaining characters may also use `_` and `-`;
- name: non-empty, at most 80 characters;
- description: optional, at most 240 characters;
- instructions: non-empty and ordered;
- first instruction: exactly `AGENTS.md`;
- every instruction: unique relative `.md` path containing no `.`, `..`, root, or prefix component;
- every source: regular UTF-8 file; symlinks are rejected.

The desktop and CLI can append a supplemental Markdown file using a previewed transaction. This updates `pack.json` and creates the new file together; an existing path is never overwritten.

## Profile

File stem and `id` must match:

```json
{
  "schemaVersion": 1,
  "id": "work",
  "name": "Work machine",
  "description": "Base safety plus work conventions.",
  "packs": [
    "base",
    "work"
  ]
}
```

`packs` must be non-empty, ordered, unique, and refer to existing Rule Pack ids.

Profile documents contain no active flag. A synced Profile can be active on one machine and inactive on another without creating a Git change.

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

Example after syncing the same source repository to two machines:

| Machine | Available Profiles | Local `activeProfileId` |
| --- | --- | --- |
| company-mac | `default`, `work`, `personal` | `work` |
| home-mac | `default`, `work`, `personal` | `personal` |

Neither selection changes `profiles/work.json` or `profiles/personal.json`.

## Runtime manifest

Runtime output is generated and must not be hand-edited:

```json
{
  "schemaVersion": 1,
  "profileId": "work",
  "profileDigest": "8d9f20b751a4",
  "renderedDigest": "f12c8a32f10b",
  "packs": [
    "base",
    "work"
  ],
  "sources": [
    {
      "packId": "base",
      "path": "AGENTS.md",
      "digest": "62a4fb4f20c1"
    },
    {
      "packId": "base",
      "path": "rules/safety.md",
      "digest": "a1d19e80bc3e"
    },
    {
      "packId": "work",
      "path": "AGENTS.md",
      "digest": "5c3fe71b9d02"
    }
  ]
}
```

The Profile digest includes logical source paths and exact contents. Renaming a source or changing its content produces another Runtime directory; `current` is switched only after the new Runtime verifies successfully.

## Git baseline

For a dedicated rule-library repository, a minimal root `.gitignore` is:

```gitignore
.runtime/
current
```

Application state and backups already live outside the repository. Credentials, chat history, provider tokens, and Agent login state must never be added to the Rule Pack library.
