# Nightmare Obfuscator

<img width="1254" height="1254" alt="nightmare-obfs" src="https://github.com/user-attachments/assets/63269c0d-793b-4092-aab3-0842712b77a8" />

Nightmare Obfuscator is a defensive IP-protection tool for controlled collaboration.
It creates a working obfuscated copy of a Rust project while the owner keeps the
original source. Each output includes a locked metadata vault with checksums,
configuration metadata, owner/project metadata, and an integrity signature.

V1 is intentionally narrow:

- Rust source support only.
- Python, JavaScript, TypeScript, Go, C, C++, and Java are roadmap-only.
- One-way obfuscation; the original repo remains the source of truth.
- No runtime key requirement for collaborators.
- No deobfuscation or encrypted original-source recovery promise.
- String encryption is disabled by default and must be enabled explicitly after
  fixture/build-parity checks.

## Install

Install from a local checkout during development:

```bash
cargo install --path .
```

Build a release binary locally:

```bash
cargo build --release
```

After release automation lands, the intended install paths are:

```bash
cargo install --git https://github.com/cdliai/nightmare-obfuscator
# or download a signed release binary from GitHub Releases
```

## Usage

Create a reusable run contract:

```bash
nightmare init \
  --source ./my-rust-project \
  --output ./partner-drop \
  --owner CDLI \
  --project my-rust-project \
  --yes
```

Run the contract and emit machine-readable stage results:

```bash
nightmare run ./nightmare.toml --json
```

Verify an obfuscated output:

```bash
nightmare verify ./partner-drop
```

Open the guided human flow when attached to a terminal:

```bash
nightmare
# or
nightmare init
```

Use `nightmare init --instant` for the reduced-motion terminal banner. Running
`nightmare init --config ./nightmare.toml` against an existing config edits only
the values supplied by flags and keeps the rest of the run contract intact.
Use `nightmare init --run` to save the config and immediately run obfuscation
through the same staged contract.

Use an owner-controlled Ed25519 signing seed when provenance identity matters:

```bash
nightmare signing public-key --signing-key ./nightmare-signing.key
nightmare verify ./partner-drop --trusted-public-key <base64-public-key>
```

The signing key file contains a base64-encoded 32-byte seed. The manifest stores
only the public verification key.

Legacy scriptable commands remain supported:

```bash
nightmare obfuscate ./my-rust-project --select src/critical
nightmare vault ./my-rust-project-obfs
```

Experimental agent planning is available as a thin surface over the same run
contract:

```bash
nightmare gate github \
  --repo https://github.com/owner/repo \
  --ref <40-character-commit-sha> \
  --config ./nightmare.toml \
  --json
```

## Behavior

By default, Nightmare copies the entire input project so build files, assets, and
configuration remain present. If no `--select` values are provided, all supported
Rust source files are obfuscated. If `--select` is provided, only matching files
or directories are obfuscated and unselected files are copied byte-for-byte.

The canonical run contract is documented in
[`docs/run-contract.md`](docs/run-contract.md). It wraps source/output paths,
owner/project metadata, selected paths, ignores, profile/intensity, feature
toggles, metadata verification, and build/smoke policy. `nightmare run --json`
separates obfuscation, metadata verification, and build/smoke checks so CI and
agents do not overclaim safety.

Language policy is documented in
[`docs/language-support.md`](docs/language-support.md). Non-Rust files are
copied as opaque assets in V1.

The default ignore set excludes `.git`, `target`, dependency/vendor folders, and
`.nightmare`. Additional `--ignore <pattern>` values are matched against relative
paths.

The output metadata lives at:

```text
.nightmare/manifest.json
.nightmare/signature
```

## Development

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Fixture acceptance tests are under `fixtures/` and `tests/acceptance.rs`.

## Governance

`main` is the stable/release branch and `dev` is the active integration branch.
CDLI.ai maintainers own repository governance through CODEOWNERS, security
reporting, and review gates.

## License

Licensed under either MIT or Apache-2.0, at your option.
