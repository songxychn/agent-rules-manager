# Security Policy

Agent Rules Manager modifies instruction files consumed by coding agents, so an unsafe write can affect later tool behavior.

Please use **Security → Report a vulnerability** on this repository when private reporting is available. If that option is unavailable, open an issue asking the maintainer for a private reporting channel without including vulnerability details. Do not post exploit details in a public issue or pull request.

Do not include real instruction files, credentials, home-directory listings, or chat data in a report. A minimal reproduction built from temporary paths is preferred. Include the affected commit or version, operating system, expected behavior, and the observed filesystem changes.

The `0.x` line is pre-release software. Before using it with valuable configuration, review the generated plan and keep the canonical rules library under your own backup or version-control workflow.
