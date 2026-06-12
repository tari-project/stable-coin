# Private Stable Coin — Issuer (with user badge)

A privacy-preserving stable coin template for the [Tari](https://www.tari.com/) network in which **every holder must
be registered with the issuer**. Token amounts are confidential (stealth resource), but only accounts holding a
non-fungible *user badge* issued by an admin may receive or hold the coin.

See [`../issuer-no-user-badge`](../issuer-no-user-badge/README.md) for the variant without holder registration, and
the [privacy tradeoff](#privacy-tradeoff-of-the-user-badge) section below for how the two differ.

## How it works

`instantiate` creates four resources and one component:

| Resource | Type | Purpose |
|---|---|---|
| Admin badge | Non-fungible | Gates all privileged operations; returned to the caller of `instantiate`. |
| User badge | Non-fungible | Per-user authentication token. Minted by admins via `create_new_user`, recallable by admins. Stores `UserData` (user id, account, created epoch) and `UserMutableData` (blacklist flag, wrapped-exchange limit). |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint/burn/recall require an admin badge. |
| Wrapped token (optional) | Public fungible | A transparent twin of the coin (`w<SYMBOL>`), exchangeable 1:1 (minus a configurable fee). See [Wrapped exchange token](#wrapped-exchange-token). |

The stable coin resource is built with `depositable`/`withdrawable` rules requiring a user or admin badge, **plus an
authorization hook** (`authorize_user_deposit`) that runs on every deposit. The hook inspects the receiving account
and panics unless that account holds a user badge in a vault — so tokens can only ever land in registered accounts.
The hook also rejects all deposits while the coin is paused.

### Method summary

Admin (requires admin badge):

- `increase_supply` / `decrease_supply` — mint into / burn from the issuer vault
- `withdraw` / `deposit` — move coins out of / into the issuer vault
- `create_new_user` / `create_new_admin` — mint badges
- `blacklist_user` / `remove_from_blacklist` — recall a user's badge into a quarantine vault (and back)
- `recall_revealed_tokens` — pull a revealed amount out of a user's account vault
- `freeze_utxos` / `unfreeze_utxos`, `burn_utxo` — UTXO-level controls
- `pause` — block all deposits of the coin
- `set_user_exchange_limit`, `set_config_transfer_fee_fixed`, `set_config_transfer_fee_percentage`

User (requires user badge proof):

- `exchange_stable_for_wrapped_tokens` — burn stable coins, mint wrapped tokens (fee applies, limited per user)
- `exchange_wrapped_for_stable_tokens` — burn wrapped tokens, mint stable coins

## Wrapped exchange token

If `enable_wrapped_token` is set at instantiation, the component creates a public fungible resource `w<SYMBOL>` with
no initial supply — it is minted and burned only through the two exchange methods. Users swap stable coins for
wrapped tokens 1:1 via `exchange_stable_for_wrapped_tokens` (the stable coins are burned, a configurable fee is
taken into the issuer vault, and the swap is capped by the user's admin-set `wrapped_exchange_limit`) and back via
`exchange_wrapped_for_stable_tokens` (no fee).

Exchanging into the wrapped token takes the value **outside the controlled stable coin**. The wrapped resource has
no recall permission, no UTXO freeze, no deposit gating, and no badge requirement — once wrapped tokens sit in a
user's vault, the issuer cannot recall or freeze them; admin authority over the wrapped resource is limited to
minting and burning through the exchange methods. The per-user exchange limit is therefore the issuer's control
point: it caps how much value each user can move out of the controlled system. The flip side for the user is that
the wrapped token is fully transparent — amounts and transfers are public, with none of the stealth resource's
confidentiality.

The feature is optional at two levels: per instance, by passing `enable_wrapped_token = false` to `instantiate`
(no wrapped resource is created and the exchange methods panic); or at the template level, by deleting the
wrapped-token code entirely (`wrapped_exchange_token.rs`, the exchange methods, and the exchange-limit
management) — mainly to reduce the compiled WASM template size.

## Privacy tradeoff of the user badge

The user badge buys the issuer **proactive compliance** at the cost of **holder privacy**. The stealth resource hides
*amounts*, but the badge mechanism makes *participation* public:

- **Holders are publicly enumerable.** The user badge is an ordinary non-fungible sitting in each user's account
  vault. Anyone scanning the ledger can list every account that holds a badge for this coin — i.e. the complete set
  of customers — even though no balance is visible.
- **Badge data is a public registry.** Each badge's on-ledger data links a `user_id` to a specific account address,
  along with its creation epoch, blacklist status, and exchange limit. If the issuer's user ids correlate with KYC
  records, this is a persistent public mapping from identity to account.
- **Deposits reveal the recipient.** The auth hook must inspect the receiving account's vaults at deposit time, so a
  transfer's destination account is visible on-ledger. The transaction graph (who pays whom, and when) is
  observable; only the amounts stay confidential.
- **Events name users.** Exchanges and admin actions emit events carrying `user_id`, adding a public activity trail
  per user.

What the issuer gains in exchange:

- **Only vetted accounts can ever hold the coin** — enforced at the resource level, not by policy. A transfer to an
  unregistered account fails atomically.
- **Blacklisting is immediate and total**: recalling a user's badge means the auth hook rejects any further deposits
  to them.
- **Pause is enforced on-chain**: the hook blocks every deposit while paused.

If holders' privacy matters more than proactive gating — and reactive controls (view key, recall, UTXO
freeze/burn) are sufficient for compliance — use
[`issuer-no-user-badge`](../issuer-no-user-badge/README.md) instead.

Note that in **both** variants the issuer holds the resource view key and can reveal amounts; confidentiality is
from the public, not from the issuer.

## Building and testing

```bash
cargo build --target wasm32-unknown-unknown --release
cargo test
```

## Manifests

- [`manifests/create_user_and_transfer.rs`](manifests/create_user_and_transfer.rs) — registers a new user (minting
  their badge) and transfers them funds in one transaction.
