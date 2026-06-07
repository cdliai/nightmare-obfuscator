# Contributing

Nightmare Obfuscator accepts changes that preserve the v1 contract: Rust-first,
build-preserving, one-way obfuscation with provenance metadata.

Before opening a pull request:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Pull requests should include focused tests when behavior changes. For obfuscation
logic, prefer fixture-based tests that prove the original and obfuscated Rust
projects both pass `cargo test`.

Branch policy:

- `main` is stable and release-oriented.
- `dev` is active integration.
- Feature branches should target `dev` unless maintainers request otherwise.

Avoid adding features that imply source recovery, runtime key requirements, or
malware-evasion use cases.
