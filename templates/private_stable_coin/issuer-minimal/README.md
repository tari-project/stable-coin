# Private Stable Coin — Issuer (minimal)

The smallest of the private stable coin templates for the [Tari](https://www.tari.com/) network. It issues a
single **stealth (confidential) coin** controlled entirely by an **admin badge**, and nothing else: no user
badges, no user registry, no blacklist, no wrapped token. Anyone can hold and transfer the coin; the issuer's
authority is *reactive* — a view key to reveal amounts, plus recall, UTXO freeze and UTXO burn.

The sibling templates build on the same core:

- [`../issuer-no-user-badge`](../issuer-no-user-badge/README.md) — adds an optional user registry and the
  wrapped exchange token.
- [`../issuer`](../issuer/README.md) — additionally restricts holding to badge-registered accounts via a deposit
  authorization hook.

Because it drops every optional feature, this template compiles to the smallest WASM of the three and is the
best starting point for a custom issuer.

## How it works

`instantiate` creates two resources and one component:

| Resource    | Type                   | Purpose                                                                                                                                                                             |
|-------------|------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Admin badge | Non-fungible (`ADM`)   | Gates every component method. One badge is minted and returned to the caller of `instantiate`; more can be minted with `create_new_admin`. Admins can recall each other's badges. |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint, burn, recall and freeze require an admin badge — **deposit and withdraw on the resource are unrestricted**. |

The component holds the issuer's `token_vault` (seeded with `initial_token_supply`), a reference to the admin
badge resource, a paused flag, and a `StableCoinConfig`.

Component access rules are `AccessRules::new().default(require_admin)`, so **every method requires an admin
badge**. The coin resource itself has no deposit/withdraw rules and no authorization hook, so transfers are
ordinary peer-to-peer stealth transfers between any two accounts — no issuer involvement and no registration
(see the `it_allows_anyone_to_receive_tokens_without_badge` test).

### Methods

| Method | Effect |
|---|---|
| `increase_supply(amount)` | Mints into the issuer vault. Blocked while paused. |
| `decrease_supply(amount)` | Withdraws from the issuer vault and burns. Blocked while paused. |
| `withdraw(amount) -> Bucket` | Moves coins out of the issuer vault. Blocked while paused. |
| `deposit(bucket)` | Moves coins into the issuer vault. Blocked while paused. |
| `recall_revealed_tokens(vault_id, amount)` | Pulls a revealed amount out of any vault into the issuer vault. |
| `burn_utxo(utxo, value_proof)` | Burns a single UTXO, given a stealth value proof. |
| `freeze_utxos(utxos)` / `unfreeze_utxos(utxos)` | UTXO-level freeze controls. |
| `create_new_admin(employee_id) -> Bucket` | Mints another admin badge, tagged with `employee_id`. |
| `pause(proof)` / `unpause(proof)` | Sets the paused flag; `proof` must be an admin badge and is recorded in the event. |
| `set_config_transfer_fee_fixed(amount)` / `set_config_transfer_fee_percentage(pct)` | Updates the configured transfer fee. |

Every method emits an event describing what it did.

### Pausing

`pause` only blocks the four vault methods above (`increase_supply`, `decrease_supply`, `withdraw`, `deposit`).
Because the coin resource has no deposit authorization hook, there is no resource-level enforcement point:
**pausing does not stop holders transferring the coin between themselves**, nor does it block recall, burn or
freeze. Treat it as an issuer-side stop on issuance and treasury movement.

### Configuration

`instantiate` takes an optional `StableCoinConfig`; passing `None` uses the default. It currently carries one
field, `transfer_fee`, which is either `FeeSpec::Fixed(Amount)` (default: `1`) or `FeeSpec::Percentage(u8)`.

Note that this template has no transfer path of its own to charge against, so the fee is **stored and
configurable but not applied anywhere** — it is a hook for issuers extending the template. `FeeSpec::calculate_fee`
implements the calculation (percentages round half up).

## Control tradeoffs

What holders get:

- **No membership list.** Holding the coin requires no badge and no registration, so the customer set cannot be
  enumerated from the ledger.
- **Confidential amounts.** Balances and transfer values are hidden from the public — though *not* from the
  issuer, who holds the resource view key.
- **Unrestricted peer-to-peer transfers.** No deposit hook intercepts transfers, so there is no resource-level
  checkpoint from which third parties can map the payment graph.

What the issuer gives up:

- **No proactive gating.** Anyone, including a sanctioned or unknown party, can receive the coin. There is no
  way to refuse a transfer before it happens.
- **No per-user controls.** With no user registry there is nothing to blacklist and no way to look up which
  vault belongs to which customer — recall and freeze operate on vault and UTXO identifiers the issuer must
  source itself (the view key is what makes them findable).
- **Pause is partial** (see above).

Choose this template when open transferability and holder privacy matter most, reactive compliance controls are
acceptable, and you want a small template to extend. Choose [`../issuer`](../issuer/README.md) when regulation
demands that only vetted accounts ever hold the token.

## Building and testing

```bash
cargo build --target wasm32-unknown-unknown --release
cargo test
```

The release profile is tuned for size (`opt-level = "s"`, LTO, `panic = "abort"`, stripped), and the template is
`no_std` with a `talc` arena allocator on WASM.

## Manifests

- [`manifests/initialize.rs`](manifests/initialize.rs) — instantiates the component (initial supply, symbol,
  metadata, divisibility, view key, config) and deposits the admin badge into the caller's account.
- [`manifests/fund.rs`](manifests/fund.rs) — withdraws from the issuer vault under an admin proof and deposits
  into a given account.
- [`manifests/transfer.rs`](manifests/transfer.rs) — creates an account and transfers funds to it from the
  issuer vault.
