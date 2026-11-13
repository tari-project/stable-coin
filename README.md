# Tari Stable Coin

A stable coin implementation on the [Tari](https://www.tari.com/) network featuring stealth (privacy-preserving) tokens,
admin controls, user management, and optional wrapped token exchange.

> **Status**: Work in progress

## Overview

This project provides Tari template smart contracts for issuing and managing a stable coin, along with a
proof-of-concept operator tool (command line and local web dashboard) for a deployed coin.

Key capabilities:
- **Stealth tokens** for privacy-preserving transfers
- **Admin controls** including pause, freeze/unfreeze UTXOs, and token recall
- **User management** with blacklisting support
- **Wrapped token exchange** allowing conversion between stealth and public fungible tokens
- **Configurable fees** (fixed or percentage-based)

## Project Structure

```
stable-coin/
├── applications/
│   └── stablecoin/       # Operator CLI and local web dashboard (proof of concept)
└── templates/
    ├── stable-coin/                          # Basic stable coin template
    ├── private_stable_coin/
    │   ├── issuer/                           # Full-featured with user badges
    │   └── issuer-no-user-badge/             # No holder registration, role-key control (working template)
    └── private_stable_coin_custom_account/   # Custom account architecture
```

## Templates

### `private_stable_coin/issuer-no-user-badge/` (Working Template)

This is the current working template. It provides a full-featured stable coin without requiring per-user
authentication badges. All privileged operations are gated by roles held by signer keys.

Features:
- Stealth token issuance with configurable supply
- Withdraw/deposit under the treasurer role
- Wrapped token exchange (stable <-> public fungible)
- User creation, blacklisting, and exchange limit management
- UTXO freeze/unfreeze and token recall
- Pause functionality

### Other Templates

The remaining templates (`stable-coin/`, `private_stable_coin/issuer/`, `private_stable_coin_custom_account/`)
are earlier iterations or alternative designs and may not be up to date.

## Building

Templates compile to WASM for deployment on the Tari network.

```bash
# Build a template
cd templates/private_stable_coin/issuer-no-user-badge
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test
```

## Operator tool

`applications/stablecoin` operates a coin deployed from the `issuer-minimal` template. It talks to an Ootle indexer
directly through `ootle-rs`, with no wallet daemon, and keeps role keys in encrypted key files.

```bash
cargo build -p stablecoin --release
```

See [applications/stablecoin/README.md](applications/stablecoin/README.md) for a walkthrough.

## License

BSD 3-Clause. See individual source files for the full license text.
