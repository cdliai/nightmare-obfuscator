# Nightmare Run Contract

Nightmare has one product contract: a versioned `nightmare.toml` file plus a
machine-readable run result. Human TUI, CI, agent gates, and future MCP tools
must call this contract instead of reimplementing obfuscation behavior.

## Config Schema

```toml
schema_version = 1
source = "./my-rust-project"
output = "./partner-drop"
profile = "balanced"
intensity = 7
selected_paths = ["src/core.rs"]
ignored_patterns = ["snapshots"]

[owner]
name = "CDLI"
contact = "security@example.com"

[project]
name = "my-rust-project"

[checks]
verify_metadata = true
build = "cargo test"

[features]
dead_code = true
flatten_control_flow = false
encrypt_strings = false
rename_identifiers = true

[signing]
private_key_path = "./nightmare-signing.key"
```

Supported profiles are `light`, `balanced`, and `aggressive`. `intensity` must
be between 1 and 10. `encrypt_strings` is disabled by default and can be enabled
only through explicit config.

`flatten_control_flow` is experimental and **not yet implemented**. It is
disabled by default; enabling it currently performs no transformation (a no-op)
and prints a warning, so a run never reports control-flow flattening that did not
happen. The toggle is retained for roadmap compatibility.

## Precedence

Nightmare resolves settings in this order:

1. Built-in defaults.
2. `NIGHTMARE_OWNER`, `NIGHTMARE_OWNER_CONTACT`, and `NIGHTMARE_PROJECT` when no
   config value is provided.
3. `nightmare.toml`.
4. Explicit CLI flags, such as `--output`, `--select`, `--ignore`, and
   `--intensity`.

`--select` replaces config selection. `--ignore` appends to the config ignores
and the built-in ignore set.

When `nightmare init` creates a contract or receives explicit `--source` or
`--output` values, relative paths are persisted as absolute paths from the
current working directory. Existing configs edited without path flags keep their
stored path values.

`[signing].private_key_path` points to a base64-encoded 32-byte Ed25519 signing
seed. The private seed is never written to the manifest. The manifest stores the
base64 Ed25519 public verification key, and `.nightmare/signature` stores the
base64 signature over the manifest bytes.

## Run Result

`nightmare run ./nightmare.toml --json` emits clean JSON on stdout. The result
uses `schema_version = 1`, has an overall `status`, records the config/source
and output paths, and reports stages in this stable order:

1. `obfuscate`
2. `verify`
3. `build`

Each stage includes `status`, `command`, `exit_code` when available,
`stderr_summary` on failures or skips, and `manifest_path` when a manifest is
available. Metadata verification is intentionally separate from build/smoke
checks so agents cannot mistake a valid manifest for a build-preserving output.

Set `checks.build = false` to disable the build/smoke stage explicitly. Omitting
the field keeps the default `cargo test` check.

`nightmare verify --trusted-public-key <base64>` verifies that the manifest was
signed by the expected owner key. Without a trusted key, verification proves the
artifact is internally signed and untampered, but does not establish owner
identity.
