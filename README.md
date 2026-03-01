# Nightmare Obfuscator

> Make code unreadable. Make it yours.

A high-intensity code obfuscation engine that transforms
your readable codebase into cryptographic nightmare fuel.
Designed for protecting intellectual property when sharing code with third parties.


---

> Quick Acknowledgement
>
> I acknowledge that some users might use the package to hide
> obfuscated malicious code snippes in public repositories.
> The purpose of this codebase is privacy and self-ownership
> of certain IPs to be safely shareable with third parties.
> I will not take any responsibility of how one use this packages.


## Features

- **Polymorphic Symbol Mangling** - Each file gets unique unreadable identifiers
- **Dead Code Injection** - Zombie functions that look real but do nothing
- **String Encryption** - Hide all string literals with XOR + base64
- **Control Flow Flattening** - Destroy readable control structures
- **Opaque Predicates** - Always-true conditions that confuse analysis
- **Owner Encryption** - AES-256-GCM recovery with master key
- **Seed Phrase Vaults** - BIP39 8-12 word access for third parties
- **Time-Locked Decryption** - Vaults unlock only after specified time
- **Self-Destruct** - Auto-corrupt after failed attempts

## Quick Start

```bash
cargo build --release

export NIGHTMARE_KEY=$(openssl rand -hex 32)

./target/release/nightmare obfuscate ./my-project --output ./obfuscated

# Create a vault for third-party access
./target/release/nightmare vault create ./secret-module.rs \
    --words 12 \
    --output ./secret.vault \
    --description "Core algorithm"

# Third party opens vault with seed phrase
./target/release/nightmare vault open ./secret.vault \
    --seed "abandon abandon ..." \
    --output ./unlocked
```

## Architecture

```
nightmare/
├── crates/
│   ├── core/          # Types, configs, errors
│   ├── crypto/        # AES-256-GCM, ChaCha20, Scrypt, BIP39
│   ├── obfuscator/    # Symbol mangler, dead code, control flow
│   ├── vault/         # Seed phrase access, timelocks
│   └── parser/        # Multi-language source parsing
└── src/
    └── main.rs        # CLI interface
```

## Security Model

1. **Owner Access** - Full decryption with `NIGHTMARE_KEY` (AES-256-GCM)
2. **Vault Access** - Third parties use BIP39 seed phrases (ChaCha20-Poly1305)
3. **Time Locks** - Vaults can be time-locked using blockchain oracles
4. **Self-Destruct** - Failed attempts decrement counter, 0 = permanent lock

## Warnings

- **IRREVERSIBLE**: Without master key, obfuscation cannot be undone
- **SEED PHRASES**: Write them down offline - lost = permanent vault lock
- **TEST FIRST**: Always test obfuscation on copies

## License

MIT - Use at your own risk. Not responsible for lost code or keys.
