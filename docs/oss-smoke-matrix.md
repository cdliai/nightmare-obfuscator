# OSS Smoke Matrix

Nightmare keeps a pinned real-project smoke matrix for build-preserving Rust
obfuscation. The matrix is intentionally separate from the fast acceptance
fixtures because it uses network checkouts and can take longer to run.

## Targets

| Target | Coverage |
| --- | --- |
| `hex` | library, tests, doctests, public APIs |
| `itoa` | small library, tests, public APIs |
| `ripgrep` | binary, workspace, features, tests |
| `serde-json` | serde, derives, tests, public APIs |
| `bitflags` | macros, derives, tests, public APIs |

Run locally with:

```bash
scripts/oss_smoke_matrix.sh --list
scripts/oss_smoke_matrix.sh --target hex
scripts/oss_smoke_matrix.sh --all
```

By default, checkouts and obfuscated outputs are written under
`$RUNNER_TEMP/nightmare-oss-smoke`, `$TMPDIR/nightmare-oss-smoke`, or
`/tmp/nightmare-oss-smoke`. Keeping them outside this repository prevents Cargo
from accidentally attaching cloned projects to Nightmare's own workspace. Set
`NIGHTMARE_SMOKE_WORKDIR` to override the location.

The workflow `.github/workflows/oss-smoke.yml` can also be run manually and is
scheduled weekly. It uses `actions/cache` for Cargo registry/git data and build
artifacts so repeated runs remain CI-friendly. Each target is fetched by pinned
full commit SHA and the checkout verifies `HEAD` before running tests.

Each target first runs the original project's normal tests and doctests. It then
writes a temporary `nightmare.toml`, runs `nightmare run --json`, and validates
that the machine-readable `obfuscate`, `verify`, and `build` stages all passed.
The run-contract build stage executes `cargo test --all-targets && cargo test
--doc` inside the obfuscated output.

## Expected Limitations

- The matrix requires network access to clone pinned OSS repositories.
- Targets are pinned to full commit SHAs to avoid surprise upstream changes.
- OSS target checks intentionally avoid `--locked` because several library
  crates do not commit a `Cargo.lock`; reproducibility comes from the pinned
  repository commits plus Cargo's own dependency resolution in CI.
- Public APIs, imports, derives, macros, and literals remain protected by the
  Rust-only v1 obfuscation policy; failures here should block enabling broader
  transforms.
- Passing this matrix does not imply non-Rust language support.
