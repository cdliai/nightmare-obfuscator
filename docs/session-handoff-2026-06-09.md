# Session Handoff - 2026-06-09

This checkpoint pauses the work before closing the remaining open issues.

## Done on Branch

- #2: Rust-only v1 language policy is implemented and tested. Non-Rust files are opaque assets, direct non-Rust selection fails, unknown `languages` config is rejected, and docs describe roadmap-only language support.
- #3: Metadata vault signatures use Ed25519 signing and verification, with trusted public-key verification and manifest re-signing tests.
- #4: String encryption is opt-in, std-only, build-preserving for the fixture, and skips unsafe Rust contexts such as constants, statics, macros, match patterns, and `if let` patterns.
- #5/#8: `nightmare.toml`, `nightmare init`, `nightmare run --json`, and the stable staged run result are implemented.
- #9: `nightmare gate github` is a thin planned adapter over the run contract, and the MCP adapter plan is documented.
- #10: Human init/TUI-facing entry points stay thin over the same contract.

## In Progress

- #6: OSS smoke matrix exists, uses pinned full SHAs, runs outside the Nightmare workspace, and validates `nightmare run --json` stages. Verified locally with `scripts/oss_smoke_matrix.sh --target hex`. Remaining next-session work: re-run advisor after the pause, consider `--all` or GitHub Actions evidence, then close when comfortable.
- #7: Live branch protection for `main` and `dev` was applied. Release workflow and CODEOWNERS fixes are local on this branch. Remaining next-session work: publish via PR/merge to `main`, confirm GitHub CODEOWNERS validation is clean, and confirm the Release workflow is visible in Actions.

## Todo Next Session

- Decide whether to close #2/#3/#4 before merge or after the branch is merged.
- Complete #6 evidence with either the full OSS matrix or a deliberate smaller acceptance policy.
- Merge/publish the release workflow and CODEOWNERS change for #7, then re-check live GitHub state.

## Verification at Pause

- `cargo fmt && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- `scripts/oss_smoke_matrix.sh --target hex`
