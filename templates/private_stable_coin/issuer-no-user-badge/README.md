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
| Admin badge | Non-fungible | Holds every role not assigned elsewhere (see [Roles](#roles)); one is returned to the caller of `instantiate`. Issued with `create_new_admin` and revoked with `revoke_admin`, both governor-only. |
| User badge | Non-fungible | An **optional registry entry**, not an access requirement. Minted by admins via `create_new_user` for known/KYC'd users; stores `UserData` (user id, account, created epoch) and `UserMutableData` (blacklist flag, wrapped-exchange limit). Used to look up a user's account for recalls and to track exchange limits. |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint, burn, recall and freeze are performed only by the component, whose methods are gated by role — **deposit and withdraw are unrestricted**. |
| Wrapped token (optional) | Public fungible | A transparent twin of the coin (`w<SYMBOL>`), exchangeable 1:1 (minus a configurable fee). See [Wrapped exchange token](#wrapped-exchange-token). |

Because the coin resource has no deposit/withdraw rules and no authorization hook, transfers are ordinary
peer-to-peer stealth transfers between any accounts (see the
`it_allows_anyone_to_receive_tokens_without_badge` test).

Every privileged method is gated by a role, including the wrapped-token exchange methods — exchanges are
facilitated by the issuer rather than called directly by end users.

## Roles

The component, not any key or badge, owns every resource it creates: the coin, the wrapped token and both badge
resources have no owner, and their mint, burn, recall and freeze rules name only the component, with locked updaters.
Every privileged action therefore goes through a component method, where the role rules and pause apply.

| Role | Methods |
|---|---|
| Governor | `set_role`, `set_role_with_proof`, `create_new_admin`, `revoke_admin`, `unpause`, `set_config_*` — and every other method, since the governor owns the component |
| Minter | `increase_supply` |
| Burner | `decrease_supply`, `burn_utxo` |
| Treasurer | `withdraw`, `deposit`, `exchange_stable_for_wrapped_tokens`, `exchange_wrapped_for_stable_tokens` (the exchanges also take the user's badge proof) |
| Compliance | `recall_revealed_tokens`, `blacklist_user`, `remove_from_blacklist`, `freeze_utxos`, `unfreeze_utxos` |
| UserManager | `create_new_user`, `set_user_exchange_limit`, `set_user_wrapped_exchange_limit` |
| Pauser | `pause` |

Each role is an access rule. `instantiate` takes an optional `RoleConfig` with one optional rule per role; a role
left unset is held by any admin badge, so passing `None` keeps a single-admin setup. A rule can name signer keys
(`public_key(..)`), specific admin badges (`non_fungible(..)`), or a threshold of either (`m_of_n(..)`), e.g. a
2-of-3 governor.

- **Rotation.** The governor reassigns a role with `set_role(role, rule)` when its rule is satisfied by the
  transaction's signers, or `set_role_with_proof(role, rule, proof)` when it requires a badge: the method body needs
  the governor's authority, and a badge is only in scope there if its proof is passed as an argument. Reassigning
  the governor also hands over ownership of the component.
- **Revocation.** Admin badges are recallable by the component, so `revoke_admin(vault_id, badge_id)` claws back
  and burns a lost badge.
- **Guards.** No role may be open to everyone, and the governor role cannot be set to `deny_all`.
- **Pause.** The pauser can pause; only the governor can unpause. While paused, minting, burning, treasury
  movements, exchanges and user registration are blocked; compliance and governance stay available so an incident
  can be contained.

Pausing does not block transfers between holders: without the deposit authorization hook of the
[`issuer`](../issuer/README.md) variant there is no resource-level enforcement point.

The resource view key is fixed at instantiation and cannot be rotated. Use a key dedicated to auditing, separate from
the keys that hold roles.

## Wrapped exchange token

If `enable_wrapped_token` is set at instantiation, the component creates a public fungible resource `w<SYMBOL>` with
no initial supply — it is minted and burned only through the two exchange methods. Stable coins are swapped for
wrapped tokens 1:1 via `exchange_stable_for_wrapped_tokens` (the stable coins are burned, a configurable fee is
taken into the issuer vault, and the swap is capped by the user's admin-set `wrapped_exchange_limit`) and back via
`exchange_wrapped_for_stable_tokens` (no fee). In this variant both methods require the treasurer role, so exchanges
are performed through the issuer.

Exchanging into the wrapped token takes the value **outside the controlled stable coin**. The wrapped resource has
no recall permission, no UTXO freeze, and no deposit gating — once wrapped tokens sit in a user's vault, the issuer
cannot recall or freeze them; the issuer's authority over the wrapped resource is limited to minting and burning
through the exchange methods. The per-user exchange limit is therefore the issuer's control point: it caps how much value
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
- **Pause does not stop transfers** (see [Roles](#roles)).

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
  metadata, divisibility, view key, wrapped-token flag, roles) and deposits the admin badge.
- [`manifests/create_user_and_transfer.rs`](manifests/create_user_and_transfer.rs) — registers a new user (minting
  their badge) and transfers them funds in one transaction.
