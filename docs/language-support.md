# Language Support

Nightmare V1 is Rust-only.

Python, JavaScript, TypeScript, Go, C, C++, and Java are roadmap-only. Their
extensions may be detected as copied assets or future identifiers, but V1 does
not obfuscate them and does not claim build-preserving support for them.

## V1 Policy

- Rust source can be obfuscated.
- Non-Rust files are copied byte-for-byte as opaque assets.
- Non-Rust files are not counted as supported source in the manifest.
- Directly selecting a non-Rust file for obfuscation fails with a Rust-only V1
  message.
- Unknown language fields in `nightmare.toml` are rejected instead of silently
  implying support.

## Future Language Readiness

Each future language needs all of the following before support is claimed:

- syntax-aware parser,
- build-preserving identifier policy,
- fixture project,
- original build/test coverage,
- obfuscated build/test coverage,
- explicit exclusions for public APIs, imports, macros, attributes,
  decorators, annotations, and literals.

Until those are in place, public APIs and language-specific constructs are out
of scope for V1 transforms.
