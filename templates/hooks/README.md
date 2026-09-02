# BridgeForge Hook Runtime

This crate is the source-distributed runtime for the managed project hooks.
`bridgeforge-codex` builds it with locked Cargo dependencies during
`init`/`adopt`/`update` and factory `git-sync` release preparation; daily hook
execution uses only the resulting binary. Pre-commit checks never build it.

Do not invoke Cargo from a hook event. The installed binary lives at
`.codex/bin/bridgeforge-hook` (or `.exe` on Windows).
