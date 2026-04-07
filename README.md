# Tari Stable Coin

A stable coin implementation on the [Tari](https://www.tari.com/) network featuring stealth (privacy-preserving) tokens,
admin controls, user management, and optional wrapped token exchange.

> **Status**: Work in progress

## Overview

This project provides Tari template smart contracts for issuing and managing a stable coin, along with a CLI and
web application for interacting with deployed instances.

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
│   ├── cli/              # Command-line interface
│   └── web/              # React + Vite web interface
└── templates/
    ├── stable-coin/                          # Basic stable coin template
    ├── private_stable_coin/
    │   ├── issuer/                           # Full-featured with user badges
    │   └── issuer-no-user-badge/             # Simplified, admin-only control (working template)
    └── private_stable_coin_custom_account/   # Custom account architecture
```

## Templates

### `private_stable_coin/issuer-no-user-badge/` (Working Template)

This is the current working template. It provides a full-featured stable coin without requiring per-user
authentication badges. All privileged operations are gated by an admin badge.

Features:
- Stealth token issuance with configurable supply
- Withdraw/deposit with admin access control
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

## CLI

The CLI interacts with a running Tari wallet daemon to build and submit transactions.

```bash
cargo build -p stable_coin_cli --release
```

Requires a Tari wallet daemon running (default: `http://localhost:9000`).

## Web Interface

See [applications/web/README.md](applications/web/README.md) for setup instructions.

## License

BSD 3-Clause. See individual source files for the full license text.
