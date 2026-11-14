# stablecoin — issuer tool (proof of concept)

A command-line tool and local web dashboard for operating a coin deployed from the
[`issuer-minimal`](../../templates/private_stable_coin/issuer-minimal/README.md) template.

It talks to an Ootle **indexer** directly through [`ootle-rs`](https://crates.io/crates/ootle-rs). There is no
wallet daemon: the tool builds, signs and submits transactions itself, and role keys live in
passphrase-encrypted key files.

> **Proof of concept.** It shows the template being operated end to end. It is not hardened for production use;
> see [Limits](#limits).

## Build

```bash
cargo build -p stablecoin --release      # from the repository root
```

## Walkthrough

Every command reads the key file (`--key`, default `issuer.key`) and the deployment file (`--deployment`,
default `deployment.json`). The passphrase is prompted for, or read from `STABLECOIN_PASSPHRASE`.

```bash
# 1. Create a key. Prints its address and public key.
stablecoin key new --network localnet

# 2. Fund it. Send public TARI to the printed address, or on a test network:
stablecoin faucet

# 3. Publish the template (once per network) and deploy a coin.
cargo build --release --target wasm32-unknown-unknown \
  --manifest-path templates/private_stable_coin/issuer-minimal/Cargo.toml
stablecoin publish templates/private_stable_coin/issuer-minimal/target/wasm32-unknown-unknown/release/tari_stablecoin_minimal.wasm
stablecoin deploy --template <template address> --symbol STC --max-supply 2000000000

# 4. Operate it.
stablecoin status
stablecoin mint 500
stablecoin send <otl_… address> 1000
stablecoin serve            # local web dashboard
```

`deploy` makes the signing key the governor, and every unset role follows the governor. Pass `--minter`,
`--burner`, `--treasurer`, `--compliance` or `--pauser` (an address or a hex public key) to give a role to another
key from the start. The signing key's view key becomes the coin's resource view key.

## Commands

| Command | Role | What it does |
|---|---|---|
| `key new` / `key show` | – | Create a key file; print its address. |
| `faucet` | – | Take test funds (test networks only). |
| `publish <wasm>` | – | Publish a template binary. |
| `deploy --template <addr>` | – | Instantiate a coin and write the deployment file. |
| `status` | – | Supply, max supply, issuer vault, roles, pause state, pending governor. |
| `mint <amount>` / `burn <amount>` | Minter / Burner | Change supply in the issuer vault. |
| `send <address> <amount>` | Treasurer | Move coins from the issuer vault to an account. |
| `recall <address> <amount>` | Compliance | Pull a revealed balance from an account back into the issuer vault. |
| `freeze <utxo>…` / `unfreeze <utxo>…` | Compliance | Freeze or unfreeze UTXOs by hex id. |
| `pause` / `unpause` | Pauser / Governor | Stop or resume issuance and treasury movements. |
| `set-role <role> <holder>` | Governor | Give a role to another key. |
| `propose-governor <holder>` | Governor | Start a governor handover. |
| `confirm-governor` | Proposed governor | Confirm it, signed by the proposed key. |
| `set-role governor <holder>` | Governor | Complete the handover after confirmation. |
| `cancel-governor` | Governor | Withdraw the proposal. |
| `set-max-supply [amount]` | Governor | Set the supply cap, or remove it. |
| `serve [--port 7400]` | – | Serve the web dashboard. |

To act as a different role, point `--key` (or `STABLECOIN_KEY`) at that role's key file. Each key pays its own
fees, so each needs a TARI balance.

## Web dashboard

`stablecoin serve` unlocks the key once and serves a single page on `127.0.0.1` with the same operations as the
command line. It prints a URL carrying a random token; every API call must present that token. The server holds
the decrypted key in memory until it is stopped.

## Key files

A key file holds an Ootle address's account and view secrets, encrypted with XChaCha20-Poly1305 under a key
derived from the passphrase with Argon2id. The address is stored in clear so `key show` works without the
passphrase. There is no recovery phrase: back the file up.

## Limits

- One signing key per transaction. A threshold (`m_of_n`) role cannot be exercised with this tool yet.
- No view-key scanning and no `burn_utxo`: finding and burning holder UTXOs is not implemented.
- `freeze` and `unfreeze` are wired up but have not been exercised against real UTXOs.
- Fees use fixed ceilings rather than an estimate.
- The dashboard is for local use by the operator. It has no user accounts and no TLS.
- Only the `issuer-minimal` template is supported.
