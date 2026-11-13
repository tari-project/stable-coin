# Private Stable Coin — Issuer (no user badge)

A privacy-preserving stable coin template for the [Tari](https://www.tari.com/) network in which **anyone can hold
and transfer the coin** — no registration with the issuer is required to receive it. Token amounts are confidential
(stealth resource) and the issuer relies on *reactive* controls (view key, recall, UTXO freeze/burn) rather than
gating who may hold the coin.

See [`../issuer`](../issuer/README.md) for the variant that restricts holding to badge-registered accounts, and the
[privacy tradeoff](#privacy-tradeoff-no-badge-vs-user-badge) section below for how the two differ.

## How it works

`instantiate` creates three resources and one component:

| Resource | Type | Purpose |
|---|---|---|
| User badge | Non-fungible | An **optional registry entry**, not an access requirement. Minted by the user manager via `create_new_user` for known/KYC'd users; stores `UserData` (user id, account, created epoch) and `UserMutableData` (blacklist flag, wrapped-exchange limit). Used to look up a user's account for recalls and to track exchange limits. |
| Stable coin | Stealth (confidential) | The coin itself. Amounts are hidden on-ledger; the issuer holds a **view key** that can reveal them. Mint, burn, recall and freeze are performed only by the component, whose methods are gated by role — **deposit and withdraw are unrestricted**. |
| Wrapped token (optional) | Public fungible | A transparent twin of the coin (`w<SYMBOL>`), exchangeable 1:1 (minus a configurable fee). See [Wrapped exchange token](#wrapped-exchange-token). |

Because the coin resource has no deposit/withdraw rules and no authorization hook, transfers are ordinary
peer-to-peer stealth transfers between any accounts (see the
`it_allows_anyone_to_receive_tokens_without_badge` test).

Every privileged method is gated by a role, including the wrapped-token exchange methods — exchanges are
facilitated by the issuer rather than called directly by end users.

## Roles

The component, not any key, owns every resource it creates: the coin, the wrapped token and the user badge
resource have no owner, and their mint, burn, recall and freeze rules name only the component, with locked updaters.
Every privileged action therefore goes through a component method, where the role rules and pause apply.

| Role | Methods |
|---|---|
| Governor | `set_role`, `propose_governor`, `cancel_governor_proposal`, `unpause`, `set_config_*` — and every other method, since the governor owns the component |
| Proposed governor | `confirm_governor` (only while a proposal is pending) |
| Minter | `increase_supply` |
| Burner | `decrease_supply`, `burn_utxo` |
| Treasurer | `withdraw`, `deposit`, `exchange_stable_for_wrapped_tokens`, `exchange_wrapped_for_stable_tokens` (the exchanges also take the user's badge proof) |
| Compliance | `recall_revealed_tokens`, `blacklist_user`, `remove_from_blacklist`, `freeze_utxos`, `unfreeze_utxos` |
| UserManager | `create_new_user`, `set_user_exchange_limit`, `set_user_wrapped_exchange_limit` |
| Pauser | `pause` |

Each role is an access rule over **signer keys**: it is satisfied by the keys that sign the transaction.
`instantiate` takes a `RoleConfig` with a required governor rule and one optional rule per other role; an unset
role follows the governor. A rule names one key (`public_key(..)`) or a combination (`any_of(..)`, `all_of(..)`,
`m_of_n(..)`), e.g. a 2-of-3 governor. Keep the governor a threshold of offline keys, and give the roles that act
often (minter, treasurer, pauser) their own keys.

- **Rotation.** The governor reassigns a role with `set_role(role, rule)`. The old rule stops working in the same
  transaction.
- **Governor handover.** Changing the governor takes three calls: the governor calls `propose_governor(rule)`,
  whoever satisfies `rule` calls `confirm_governor()`, and the governor then calls `set_role(Governor, rule)` with
  the same rule, which also hands over ownership of the component. `cancel_governor_proposal()` withdraws a
  proposal. This stops a handover to a rule no one holds, such as a mistyped key. The governor
  owns the component and can confirm its own proposal, so the check guards against mistakes, not against a
  governor acting deliberately. A confirmation does not expire: complete the handover promptly, and cancel and
  re-propose if the new governor's keys may have changed since it confirmed.
- **Guards.** A role rule may name only signer keys. Anything else (`allow_all`, any resource or badge, including
  user badges, and caller components) is rejected, so a role cannot go to holders the issuer never chose. The
  governor role cannot be set to `deny_all`.
- **Pause.** The pauser can pause; only the governor can unpause. While paused, minting, burning, treasury
  movements, exchanges and user registration are blocked; compliance and governance stay available so an incident
  can be contained.

Pausing does not block transfers between holders: without the deposit authorization hook of the
[`issuer`](../issuer/README.md) variant there is no resource-level enforcement point.

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

### Incident response

If a role's key is compromised, the governor reassigns every affected role in one transaction (several
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
no initial supply — it is minted and burned only through the two exchange methods. Stable coins are swapped for
wrapped tokens 1:1 via `exchange_stable_for_wrapped_tokens` (the stable coins are burned, a configurable fee is
taken into the issuer vault, and the swap is capped by the user's `wrapped_exchange_limit`, set by the user manager,) and back via
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
  metadata, divisibility, wrapped-token flag) with the view key and `RoleConfig` passed in as variables.
- [`manifests/create_user_and_transfer.rs`](manifests/create_user_and_transfer.rs) — registers a new user (minting
  their badge) and transfers them funds in one transaction.

## Integrating

Identify the coin by its **resource address** and the issuer by its **component address**. The template is public:
anyone can instantiate a coin with the same symbol and metadata, so neither identifies the issuer.
