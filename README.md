# Nightmare Obfuscator

<img width="1254" height="1254" alt="nightmare-obfs" src="https://github.com/user-attachments/assets/63269c0d-793b-4092-aab3-0842712b77a8" />

Nightmare Obfuscator is a defensive IP-protection tool for controlled collaboration.
It creates a working obfuscated copy of a Rust project while the owner keeps the
original source. Each output includes a locked metadata vault with checksums,
configuration metadata, owner/project metadata, and an integrity signature.

V1 is intentionally narrow:

- Rust source support only.
- One-way obfuscation; the original repo remains the source of truth.
- No runtime key requirement for collaborators.
- No deobfuscation or encrypted original-source recovery promise.
- String encryption is disabled until it can preserve builds reliably.

## Install

```bash
cargo build --release
```

## Usage

Obfuscate a project into a sibling `<name>-obfs` directory:

```bash
nightmare obfuscate ./my-rust-project
```

Copy the full project but obfuscate only selected paths:

```bash
nightmare obfuscate ./my-rust-project --select src/critical
```

Choose an explicit output directory and ignore additional paths:

```bash
nightmare obfuscate ./my-rust-project \
  --output ./partner-drop \
  --select src/core.rs \
  --ignore snapshots
```

Verify an obfuscated output:

```bash
nightmare verify ./my-rust-project-obfs
```

Inspect the metadata vault:

```bash
nightmare vault ./my-rust-project-obfs
```

## Behavior

By default, Nightmare copies the entire input project so build files, assets, and
configuration remain present. If no `--select` values are provided, all supported
Rust source files are obfuscated. If `--select` is provided, only matching files
or directories are obfuscated and unselected files are copied byte-for-byte.

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
