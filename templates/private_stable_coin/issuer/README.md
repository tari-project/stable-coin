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
| Admin badge | Non-fungible | Badge #0 is returned to the caller of `instantiate` and holds every role not assigned elsewhere (see [Roles](#roles)). Badges issued with `create_new_admin` hold no role until a rule names them. Issuing and revoking (`revoke_admin`) are governor-only. |
| User badge | Non-fungible | Per-user authentication token. Minted via `create_new_user` and recalled via `blacklist_user`, both role-gated component methods. Stores `UserData` (user id, account, created epoch) and `UserMutableData` (blacklist flag, wrapped-exchange limit). |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint, burn, recall and freeze are performed only by the component, whose methods are gated by role. |
| Wrapped token (optional) | Public fungible | A transparent twin of the coin (`w<SYMBOL>`), exchangeable 1:1 (minus a configurable fee). See [Wrapped exchange token](#wrapped-exchange-token). |

Who may hold the coin is enforced by an **authorization hook** (`authorize_user_deposit`) that runs on every
deposit. The hook inspects the receiving account and panics unless that account holds a user badge in a vault — so
tokens can only ever land in registered accounts. The hook also rejects all deposits while the coin is paused.

The hook, rather than a `depositable`/`withdrawable` access rule, is what does the gating: a deposit is authorized
inside the receiving account's call frame, where no proof is in scope, so a rule requiring a badge proof could
never be satisfied there. The hook runs in that frame and reads the account's vaults instead.

## Roles

The component, not any key or badge, owns every resource it creates: the coin, the wrapped token and both badge
resources have no owner, and their mint, burn, recall and freeze rules name only the component, with locked updaters.
Every privileged action therefore goes through a component method, where the role rules and pause apply.

| Role | Methods |
|---|---|
| Governor | `set_role`, `propose_governor`, `cancel_governor_proposal`, `create_new_admin`, `revoke_admin`, `unpause`, `set_config_*` — and every other method, since the governor owns the component |
| Proposed governor | `confirm_governor` (only while a proposal is pending) |
| Minter | `increase_supply` |
| Burner | `decrease_supply`, `burn_utxo` |
| Treasurer | `withdraw`, `deposit`; may also call the exchange methods, which still require the user's badge proof |
| Compliance | `recall_revealed_tokens`, `blacklist_user`, `remove_from_blacklist`, `freeze_utxos`, `unfreeze_utxos` |
| UserManager | `create_new_user`, `set_user_exchange_limit`, `set_user_wrapped_exchange_limit` |
| Pauser | `pause` |

Users call the exchange methods with their user badge proof:

- `exchange_stable_for_wrapped_tokens` — burn stable coins, mint wrapped tokens (fee applies, limited per user)
- `exchange_wrapped_for_stable_tokens` — burn wrapped tokens, mint stable coins

Each role is an access rule. `instantiate` takes an optional `RoleConfig` with one optional rule per role. An unset
governor is admin badge #0, the badge `instantiate` returns, and every other unset role follows the governor, so
passing `None` keeps a single-admin setup. Badges issued later with `create_new_admin` hold no role until a rule
names them, one by one (`non_fungible(..)`) or all together (`resource(admin_badge)`). A rule can name signer keys
(`public_key(..)`), specific admin badges (`non_fungible(..)`), or a threshold of either (`m_of_n(..)`), e.g. a
2-of-3 governor.

- **Rotation.** The governor reassigns a role with `set_role(role, rule)`. A badge governor authorizes its badge
  proof in the transaction (`create_proof`, then `authorize_proof`); the engine lends authorized proofs to the
  method, where the component's ownership check runs. A threshold across several parties is better expressed over
  their signer keys than over badges held in separate accounts.
- **Governor handover.** Changing the governor takes three calls: the governor calls `propose_governor(rule)`,
  whoever satisfies `rule` calls `confirm_governor()`, and the governor then calls `set_role(Governor, rule)` with
  the same rule, which also hands over ownership of the component. `cancel_governor_proposal()` withdraws a
  proposal. This stops a handover to a rule no one holds, such as a mistyped key or an unissued badge. The governor
  owns the component and can confirm its own proposal, so the check guards against mistakes, not against a
  governor acting deliberately. A confirmation does not expire: complete the handover promptly, and cancel and
  re-propose if the new governor's keys may have changed since it confirmed.
- **Revocation.** Admin badges are recallable by the component, so `revoke_admin(vault_id, badge_id)` claws back
  and burns a lost badge. It refuses a badge without which the governor rule, or a pending governor proposal, could
  no longer be satisfied: a stolen badge in an `any_of` or `m_of_n` governor rule can be revoked straight away while
  the other badges still satisfy it, but the last badge able to govern cannot.
- **Guards.** A role rule may name only signer keys (`public_key(..)`), specific admin badges (`non_fungible(..)`)
  and every admin badge (`resource(admin_badge)`), combined with `any_of`, `all_of` and `m_of_n`. Anything else
  (`allow_all`, the coin, TARI, the wrapped token, user badges, caller components) is rejected, so a role cannot go to
  holders the issuer never chose. The governor role cannot be set to `deny_all`. These guards do not stop a governor
  that deliberately confirms its own proposal for a rule no one holds. If that happens no role can change again, and
  a paused component stays paused.
- **Pause.** The pauser can pause; only the governor can unpause. While paused, the hook rejects every deposit of the
  coin, and minting, burning, treasury movements, exchanges and user registration are blocked; compliance and
  governance stay available so an incident can be contained.

The resource view key is fixed at instantiation and cannot be rotated. Use a key dedicated to auditing, separate from
the keys that hold roles.

### Incident response

If a role's key or badge is compromised, the governor reassigns every affected role in one transaction (several
`set_role` calls). They take effect together, so the compromised holder has no window between them to act. Pause
blocks issuance and treasury movements but not recall or freeze: a compromised compliance role keeps those until it
is reassigned.

## Configuration

The component starts with the default `StableCoinConfig`; the governor changes it with the `set_config_*` methods.

| Field | Default | Effect |
|---|---|---|
| `transfer_fee` | `FeeSpec::Fixed(1)` | Set with `set_config_transfer_fee_fixed` / `set_config_transfer_fee_percentage`. |
| `wrapped_exchange_fee` | `FeeSpec::Percentage(1)` | Taken by `exchange_stable_for_wrapped_tokens`. |
| `default_exchange_limit` | `1000` | Each new user's wrapped exchange limit. |
| `max_supply` | `None` | Caps the coin plus the wrapped token outstanding: `increase_supply` fails past it. Exchanges never raise that total. Set or remove it with `set_config_max_supply`, which refuses a value below the current total. |

`freeze_utxos` and `unfreeze_utxos` take at most 100 UTXOs per call.

## Wrapped exchange token

If `enable_wrapped_token` is set at instantiation, the component creates a public fungible resource `w<SYMBOL>` with
no initial supply — it is minted and burned only through the two exchange methods. Users swap stable coins for
wrapped tokens 1:1 via `exchange_stable_for_wrapped_tokens` (the stable coins are burned, a configurable fee is
taken into the issuer vault, and the swap is capped by the user's issuer-set `wrapped_exchange_limit`) and back via
`exchange_wrapped_for_stable_tokens` (no fee).

Exchanging into the wrapped token takes the value **outside the controlled stable coin**. The wrapped resource has
no recall permission, no UTXO freeze, no deposit gating, and no badge requirement — once wrapped tokens sit in a
user's vault, the issuer cannot recall or freeze them; the issuer's authority over the wrapped resource is limited
to minting and burning through the exchange methods. The per-user exchange limit is therefore the issuer's control
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
- **Events name users.** Exchanges and issuer actions emit events carrying `user_id`, adding a public activity trail
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
