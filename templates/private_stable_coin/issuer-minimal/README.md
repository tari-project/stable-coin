# Private Stable Coin — Issuer (minimal)

The smallest of the private stable coin templates for the [Tari](https://www.tari.com/) network. It issues a
single **stealth (confidential) coin** controlled by **role keys**, and nothing else: no badges, no user
registry, no blacklist, no wrapped token. Anyone can hold and transfer the coin; the issuer's
authority is *reactive* — a view key to reveal amounts, plus recall, UTXO freeze and UTXO burn.

The sibling templates build on the same core:

- [`../issuer-no-user-badge`](../issuer-no-user-badge/README.md) — adds an optional user registry and the
  wrapped exchange token.
- [`../issuer`](../issuer/README.md) — additionally restricts holding to badge-registered accounts via a deposit
  authorization hook.

Because it drops every optional feature, this template compiles to the smallest WASM of the three and is the
best starting point for a custom issuer.

## How it works

`instantiate` creates one resource and one component:

| Resource    | Type                   | Purpose |
|-------------|------------------------|---------|
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint, burn, recall and freeze are performed only by the component, whose methods are gated by role — **deposit and withdraw on the resource are unrestricted**. |

The component holds the issuer's `token_vault` (seeded with `initial_token_supply`), a paused flag, a
`StableCoinConfig` and the rule for each role.

Every method is gated by a role (see [Roles](#roles)). The coin resource itself has no deposit/withdraw rules and
no authorization hook, so transfers are ordinary peer-to-peer stealth transfers between any two accounts — no
issuer involvement and no registration (see the `it_allows_anyone_to_hold_and_transfer_the_coin` test).

### Methods

| Method | Role | Effect |
|---|---|---|
| `increase_supply(amount)` | Minter | Mints into the issuer vault. Blocked while paused. |
| `decrease_supply(amount)` | Burner | Withdraws from the issuer vault and burns. Blocked while paused. |
| `withdraw(amount) -> Bucket` | Treasurer | Moves coins out of the issuer vault. Blocked while paused. |
| `deposit(bucket)` | Treasurer | Moves coins into the issuer vault. Blocked while paused. |
| `recall_revealed_tokens(vault_id, amount)` | Compliance | Pulls a revealed amount out of any vault into the issuer vault. |
| `burn_utxo(utxo, value_proof)` | Burner | Burns a single UTXO, given a stealth value proof. Blocked while paused. |
| `freeze_utxos(utxos)` / `unfreeze_utxos(utxos)` | Compliance | UTXO-level freeze controls, at most 100 UTXOs per call. |
| `pause()` / `unpause()` | Pauser / Governor | Sets or clears the paused flag. |
| `set_role(role, rule)` | Governor | Reassigns a role (see [Roles](#roles)). Changing the governor needs a confirmed proposal. |
| `propose_governor(rule)` / `cancel_governor_proposal()` | Governor | Proposes the next governor, or withdraws the proposal. |
| `confirm_governor()` | Proposed governor | Confirms the pending proposal. |
| `set_config_max_supply(max)` | Governor | Sets or removes (`None`) the supply cap. Cannot go below the current supply. |

Every method emits an event named after the method (`TariStableCoin.<method>`); amounts are under the `amount` key
and config changes carry `prev` and `new`. Events do not name the signer: the transaction records who signed it, and
the privileged methods do not depend on the transaction having a main signer.

### Roles

The component, not any key, owns the coin resource: it has no owner, and its mint, burn, recall and freeze rules
name only the component, with locked updaters. Every privileged action therefore goes through a component method,
where the role rules and pause apply. The governor owns the component, so it can also call every other method.

Each role is an access rule over **signer keys**: it is satisfied by the keys that sign the transaction.
`instantiate` takes a `RoleConfig` with a required governor rule and one optional rule per other role; an unset
role follows the governor. A rule names one key (`public_key(..)`) or a combination (`any_of(..)`, `all_of(..)`,
`m_of_n(..)`), e.g. a 2-of-3 governor.

| Role | Does | Suggested custody |
|---|---|---|
| Governor | Reassigns roles, changes config, unpauses. Owns the component. | A threshold of offline keys. |
| Minter | `increase_supply` | Separate from the treasurer. |
| Burner | `decrease_supply`, `burn_utxo` | Separate from the view key holder. |
| Treasurer | `withdraw`, `deposit` | Operational key. |
| Compliance | Recall, freeze, unfreeze | Compliance team key. |
| Pauser | `pause` | A key that can act quickly. |

- **Rotation.** The governor reassigns a role with `set_role(role, rule)`. The old rule stops working in the same
  transaction.
- **Governor handover.** Changing the governor takes three calls: the governor calls `propose_governor(rule)`,
  whoever satisfies `rule` calls `confirm_governor()`, and the governor then calls `set_role(Governor, rule)` with
  the same rule, which also hands over ownership of the component. `cancel_governor_proposal()` withdraws a
  proposal. This stops a handover to a rule no one holds, such as a mistyped key. The governor owns the component
  and can confirm its own proposal, so the check guards against mistakes, not against a governor acting
  deliberately. A confirmation does not expire: complete the handover promptly, and cancel and re-propose if the
  new governor's keys may have changed since it confirmed.
- **Guards.** A role rule may name only signer keys. Anything else (`allow_all`, any resource or badge, caller
  components) is rejected, so a role cannot go to holders the issuer never chose. The governor role cannot be set
  to `deny_all`.

### Threat model

The template assumes the role holders are the issuer and are trusted. It does not defend holders against the
issuer: the compliance role can recall and freeze, and the burner can burn any UTXO with the view key.

What it is built to limit is **key compromise and operator error**:

- A stolen **non-governor** key is contained: the governor reassigns that role and the key stops working. Until
  then it can do everything its role allows; `max_supply` bounds what a stolen minter key can mint.
- A stolen **governor** key is not recoverable. Nothing sits above the governor: whoever satisfies its rule can
  hand the component over to themselves. Use a threshold rule and keep the keys offline.
- A governor rule that no one can satisfy is equally permanent: no role can change again and a paused component
  stays paused. The handover confirmation guards against doing this by mistake.

### View key

The resource view key is fixed at instantiation and **cannot be rotated**. Its secret reveals every amount of the
coin, and together with the burner role it can burn **any holder's UTXO** without that holder's signature. Treat it
as a privileged credential, not just an audit key: generate it for this coin, keep it offline, and keep it separate
from the keys that hold roles.

### Pausing

The pauser can pause; only the governor can unpause. `pause` blocks the vault methods above (`increase_supply`,
`decrease_supply`, `withdraw`, `deposit`) and `burn_utxo`. Because the coin resource has no deposit authorization
hook, there is no resource-level enforcement point: **pausing does not stop holders transferring the coin between
themselves**, nor does it block recall or freeze, which stay available to contain an incident. Treat it as an
issuer-side stop on issuance and treasury movement.

### Configuration

`instantiate` takes an optional `StableCoinConfig`; passing `None` uses the default. The governor changes it later
with `set_config_max_supply`.

| Field | Default | Effect |
|---|---|---|
| `max_supply` | `None` | Caps the total supply: `increase_supply` fails past it, and `instantiate` rejects a larger initial supply. `None` leaves supply uncapped. |

### Incident response

If a role's key is compromised, the governor reassigns every affected role in one transaction (several
`set_role` calls). They take effect together, so the compromised holder has no window between them to act. Pause
blocks issuance and treasury movements but not recall or freeze: a compromised compliance role keeps those until it
is reassigned.

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
  metadata, divisibility, config) with the view key and `RoleConfig` passed in as variables.
- [`manifests/fund.rs`](manifests/fund.rs) — withdraws from the issuer vault, signed by a treasurer key, and
  deposits into a given account.
- [`manifests/transfer.rs`](manifests/transfer.rs) — creates an account and transfers funds to it from the
  issuer vault.

## Integrating

Identify the coin by its **resource address** and the issuer by its **component address**. The template is public:
anyone can instantiate a coin with the same symbol and metadata, so neither identifies the issuer.
