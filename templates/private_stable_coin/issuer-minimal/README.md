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
| Admin badge | Non-fungible (`ADM`)   | Holds every role not assigned elsewhere (see [Roles](#roles)). One badge is minted and returned to the caller of `instantiate`; the governor issues more with `create_new_admin` and revokes them with `revoke_admin`. |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint, burn, recall and freeze are performed only by the component, whose methods are gated by role — **deposit and withdraw on the resource are unrestricted**. |

The component holds the issuer's `token_vault` (seeded with `initial_token_supply`), a reference to the admin
badge resource, a paused flag, a `StableCoinConfig` and the rule for each role.

Every method is gated by a role (see [Roles](#roles)). The coin resource itself has no deposit/withdraw rules and no authorization hook, so transfers are
ordinary peer-to-peer stealth transfers between any two accounts — no issuer involvement and no registration
(see the `it_allows_anyone_to_receive_tokens_without_badge` test).

### Methods

| Method | Role | Effect |
|---|---|---|
| `increase_supply(amount)` | Minter | Mints into the issuer vault. Blocked while paused. |
| `decrease_supply(amount)` | Burner | Withdraws from the issuer vault and burns. Blocked while paused. |
| `withdraw(amount) -> Bucket` | Treasurer | Moves coins out of the issuer vault. Blocked while paused. |
| `deposit(bucket)` | Treasurer | Moves coins into the issuer vault. Blocked while paused. |
| `recall_revealed_tokens(vault_id, amount)` | Compliance | Pulls a revealed amount out of any vault into the issuer vault. |
| `burn_utxo(utxo, value_proof)` | Burner | Burns a single UTXO, given a stealth value proof. Blocked while paused. |
| `freeze_utxos(utxos)` / `unfreeze_utxos(utxos)` | Compliance | UTXO-level freeze controls. |
| `create_new_admin(employee_id) -> Bucket` | Governor | Mints another admin badge, tagged with `employee_id`. |
| `revoke_admin(vault_id, badge_id)` | Governor | Recalls an admin badge and burns it. |
| `pause()` / `unpause()` | Pauser / Governor | Sets or clears the paused flag. |
| `set_role(role, rule)` / `set_role_with_proof(role, rule, proof)` | Governor | Reassigns a role (see [Roles](#roles)). |
| `set_config_transfer_fee_fixed(amount)` / `set_config_transfer_fee_percentage(pct)` | Governor | Updates the configured transfer fee. |

Every method emits an event describing what it did.

### Roles

The component, not any key or badge, owns both resources: they have no owner, and their mint, burn, recall and
freeze rules name only the component, with locked updaters. Every privileged action therefore goes through a
component method, where the role rules and pause apply. The governor owns the component, so it can also call every
other method.

Each role is an access rule. `instantiate` takes an optional `RoleConfig` with one optional rule per role; a role
left unset is held by any admin badge, so passing `None` keeps a single-admin setup. The governor must be set
whenever another role is: left at the default, every admin badge would govern, and the governor can call every
method. A rule can name signer keys
(`public_key(..)`), specific admin badges (`non_fungible(..)`), or a threshold of either (`m_of_n(..)`), e.g. a
2-of-3 governor.

- **Rotation.** The governor reassigns a role with `set_role(role, rule)` when its rule is satisfied by the
  transaction's signers, or `set_role_with_proof(role, rule, proof)` when it requires a badge: the method body needs
  the governor's authority, and a badge is only in scope there if its proof is passed as an argument. Reassigning
  the governor also hands over ownership of the component. A badge threshold governor must present all of its
  badges in that one proof, so they must sit in one account; a threshold across several parties is better
  expressed over their signer keys.
- **Revocation.** Admin badges are recallable by the component, so `revoke_admin(vault_id, badge_id)` claws back
  and burns a lost badge.
- **Guards.** No role may be open to everyone, and the governor role cannot be set to `deny_all`. These guards
  do not stop the governor making itself unreachable in other ways, such as revoking the last admin badge that
  satisfies its rule or naming a key nobody holds. If that happens no role can change again, and a paused
  component stays paused.

The resource view key is fixed at instantiation and cannot be rotated. Use a key dedicated to auditing, separate from
the keys that hold roles.

### Pausing

The pauser can pause; only the governor can unpause. `pause` blocks the vault methods above (`increase_supply`,
`decrease_supply`, `withdraw`, `deposit`) and `burn_utxo`. Because the coin resource has no deposit authorization
hook, there is no resource-level enforcement point: **pausing does not stop holders transferring the coin between
themselves**, nor does it block recall or freeze, which stay available to contain an incident. Treat it as an
issuer-side stop on issuance and treasury movement.

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
  metadata, divisibility, view key, config, roles) and deposits the admin badge into the caller's account.
- [`manifests/fund.rs`](manifests/fund.rs) — withdraws from the issuer vault under a treasurer proof and deposits
  into a given account.
- [`manifests/transfer.rs`](manifests/transfer.rs) — creates an account and transfers funds to it from the
  issuer vault.
