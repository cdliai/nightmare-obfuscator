# MCP Adapter Plan

The MCP server is a thin protocol adapter over `nightmare gate` and
`nightmare run`. It must not contain a second obfuscation engine, a second
config resolver, or TUI automation.

## Planned Tools

- `nightmare_plan_obfuscation`: validates a `nightmare.toml` contract and
  returns the normalized plan.
- `nightmare_run_gate`: calls `nightmare gate github --repo ... --ref ...
  --config ... --json`.
- `nightmare_get_run_status`: reads a stored run result by id or path.
- `nightmare_read_manifest`: reads `.nightmare/manifest.json` from an output.

## Safety Boundaries

Agentic GitHub gates must use immutable commit SHAs, disposable workspaces, and
structured stage output. Fetching, obfuscation, metadata verification, and
build/smoke execution are separate stages. Untrusted build scripts are not run
by the current plan-only stub; enabling them later requires an explicit
sandbox/execution policy.

Gate planning must reflect the loaded run contract. If `verify_metadata` is
false, the gate marks the verify stage skipped. If `build` is customized or
disabled, the gate reports that exact command or skip reason instead of assuming
`cargo test`.

## Contract Mapping

Every MCP response should include the reproducible CLI command it maps to. A
future MCP call should be explainable as one of these commands:

```bash
nightmare run ./nightmare.toml --json
nightmare gate github --repo https://github.com/owner/repo --ref <sha> --config ./nightmare.toml --json
nightmare verify ./partner-drop
```

This keeps MCP, CI, agents, and the human TUI aligned on the same core result
schema.
