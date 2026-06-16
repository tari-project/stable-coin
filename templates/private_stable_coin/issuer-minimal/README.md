# Private Stable Coin — Issuer (no user badge)

A privacy-preserving stable coin template for the [Tari](https://www.tari.com/) network in which **anyone can hold
and transfer the coin** — no registration with the issuer is required to receive it. Token amounts are confidential
(stealth resource) and the issuer relies on *reactive* controls (view key, recall, UTXO freeze/burn) rather than
gating who may hold the coin.

See [`../issuer`](../issuer/README.md) for the variant that restricts holding to badge-registered accounts, and the
[privacy tradeoff](#privacy-tradeoff-no-badge-vs-user-badge) section below for how the two differ.

## How it works

`instantiate` creates four resources and one component:

| Resource | Type | Purpose |
|---|---|---|
| Admin badge | Non-fungible | Gates all privileged operations; returned to the caller of `instantiate`. |
| User badge | Non-fungible | An **optional registry entry**, not an access requirement. Minted by admins via `create_new_user` for known/KYC'd users; stores `UserData` (user id, account, created epoch) and `UserMutableData` (blacklist flag, wrapped-exchange limit). Used to look up a user's account for recalls and to track exchange limits. |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint/burn/recall require an admin badge — **deposit and withdraw are unrestricted**. |
| Wrapped token (optional) | Public fungible | A transparent twin of the coin (`w<SYMBOL>`), exchangeable 1:1 (minus a configurable fee). See [Wrapped exchange token](#wrapped-exchange-token). |

Because the coin resource has no deposit/withdraw rules and no authorization hook, transfers are ordinary
peer-to-peer stealth transfers between any accounts (see the
`it_allows_anyone_to_receive_tokens_without_badge` test).

All component methods default to requiring an admin badge, including the wrapped-token exchange methods — exchanges
are facilitated by the issuer rather than called directly by end users.

### Method summary

All methods require an admin badge:

- `increase_supply` / `decrease_supply` — mint into / burn from the issuer vault
- `withdraw` / `deposit` — move coins out of / into the issuer vault
- `create_new_user` / `create_new_admin` — mint badges
- `blacklist_user` / `remove_from_blacklist` — recall a registered user's badge into a quarantine vault (and back)
- `recall_revealed_tokens` — pull a revealed amount out of a registered user's account vault
- `freeze_utxos` / `unfreeze_utxos`, `burn_utxo` — UTXO-level controls
- `exchange_stable_for_wrapped_tokens` / `exchange_wrapped_for_stable_tokens` — convert between the stealth coin and
  the public wrapped token (requires the user's badge proof in addition to admin access)
- `pause` — records a paused flag and emits an event. Note: without the deposit authorization hook of the
  [`issuer`](../issuer/README.md) variant there is no resource-level enforcement point, so pausing does not block
  transfers on-chain.
- `set_user_exchange_limit`, `set_config_transfer_fee_fixed`, `set_config_transfer_fee_percentage`

## Wrapped exchange token

If `enable_wrapped_token` is set at instantiation, the component creates a public fungible resource `w<SYMBOL>` with
no initial supply — it is minted and burned only through the two exchange methods. Stable coins are swapped for
wrapped tokens 1:1 via `exchange_stable_for_wrapped_tokens` (the stable coins are burned, a configurable fee is
taken into the issuer vault, and the swap is capped by the user's admin-set `wrapped_exchange_limit`) and back via
`exchange_wrapped_for_stable_tokens` (no fee). In this variant both methods are admin-gated, so exchanges are
performed through the issuer.

Exchanging into the wrapped token takes the value **outside the controlled stable coin**. The wrapped resource has
no recall permission, no UTXO freeze, and no deposit gating — once wrapped tokens sit in a user's vault, the issuer
cannot recall or freeze them; admin authority over the wrapped resource is limited to minting and burning through
the exchange methods. The per-user exchange limit is therefore the issuer's control point: it caps how much value
each user can move out of the controlled system. The flip side for the user is that the wrapped token is fully
transparent — amounts and transfers are public, with none of the stealth resource's confidentiality.

The feature is optional at two levels: per instance, by passing `enable_wrapped_token = false` to `instantiate`
(no wrapped resource is created and the exchange methods panic); or at the template level, by deleting the
wrapped-token code entirely (`wrapped_exchange_token.rs`, the exchange methods, and the exchange-limit
management) — mainly to reduce the compiled WASM template size.

## Privacy tradeoff: no badge vs. user badge

This variant trades issuer control for holder privacy. Compared to the
[user-badge variant](../issuer/README.md):

What holders gain:

- **No public membership list.** In the badge variant, every holder carries a publicly visible badge NFT in their
  account, so the complete customer set can be enumerated by anyone scanning the ledger. Here, an account holding
  the coin carries no public marker beyond having a vault for the resource, and receiving the coin requires no
  issuer involvement at all.
- **No public identity registry.** Badges (where issued) still record `user_id` → account on-ledger, but only for
  users the issuer chooses to register — not as a precondition for holding the coin.
- **Unrestricted peer-to-peer transfers.** Deposits aren't intercepted by an authorization hook that inspects the
  receiving account, so third parties can't rely on a resource-level checkpoint to map the payment graph.

What the issuer gives up:

- **No proactive gating.** Anyone, including a sanctioned or unknown party, can receive the coin. In the badge
  variant a transfer to an unregistered account fails atomically; here it succeeds.
- **Blacklisting is bookkeeping, not enforcement.** Recalling a user's badge updates the registry but does not stop
  the account from continuing to send and receive the coin. Enforcement must instead use the reactive tools:
  reveal amounts via the **view key**, `recall_revealed_tokens`, `freeze_utxos`, or `burn_utxo`.
- **Pause is advisory** (see method summary above).

Note that in **both** variants the issuer holds the resource view key and can reveal amounts; confidentiality is
from the public, not from the issuer.

Choose this variant when open transferability and holder privacy matter most and reactive compliance controls are
acceptable; choose [`issuer`](../issuer/README.md) when regulation demands that only vetted accounts ever hold the
token.

## Building and testing

```bash
cargo build --target wasm32-unknown-unknown --release
cargo test
```

## Manifests

- [`manifests/initialize.rs`](manifests/initialize.rs) — instantiates the component (initial supply, symbol,
  metadata, divisibility, view key, wrapped-token flag) and deposits the admin badge.
- [`manifests/create_user_and_transfer.rs`](manifests/create_user_and_transfer.rs) — registers a new user (minting
  their badge) and transfers them funds in one transaction.
