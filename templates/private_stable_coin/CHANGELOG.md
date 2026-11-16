# Changelog

Changes to the `private_stable_coin` templates (`issuer-minimal`, `issuer-no-user-badge` and `issuer`).
Entries apply to all three unless a template is named.

## Unreleased

Changes since [`v0.1.0`](https://github.com/tari-project/stable-coin/tree/v0.1.0). This is a breaking
release: a coin instantiated from `v0.1.0` is not compatible with tooling written for these templates, and
existing manifests and event consumers need updating.

### Breaking

- **Roles are held by signer keys; the admin badge is removed.**
  ([#46](https://github.com/tari-project/stable-coin/pull/46))
  - `instantiate` takes `roles: RoleConfig` instead of `Option<RoleConfig>` and returns the component instead
    of an admin badge bucket.
  - `RoleConfig.governor` is required. Every other unset role follows the governor.
  - A role rule may name only signer keys (`public_key(..)`), combined with `any_of`, `all_of` and `m_of_n`.
    Rules naming a resource, a badge or a caller are rejected, as are `allow_all` and a `deny_all` governor.
  - Removed `create_new_admin`, `revoke_admin` and `set_role_with_proof`. A role is taken away from a key by
    reassigning it with `set_role`.
- **Changing the governor needs confirmation.** ([#44](https://github.com/tari-project/stable-coin/pull/44))
  `set_role(Governor, rule)` is rejected unless the governor first called `propose_governor(rule)` and a
  holder of `rule` called `confirm_governor()`. `cancel_governor_proposal()` withdraws a proposal.
- **Events are renamed.** ([#46](https://github.com/tari-project/stable-coin/pull/46))
  - Every event topic is the name of the method that emits it. Renamed: `recall_tokens` →
    `recall_revealed_tokens`, `admin.paused` → `pause`, `admin.unpaused` → `unpause`, `admin.freeze_utxos` →
    `freeze_utxos`, `admin.unfreeze_utxos` → `unfreeze_utxos`, and `config.set_*` → `set_config_*`.
  - Amounts are under the `amount` key. Config events carry `prev` and `new`.
  - Freeze and unfreeze events carry the UTXO ids instead of a count.
  - Events no longer include `tx_signer` (or `admin` on `set_user_exchange_limit`).
- **`issuer-minimal`: the transfer fee config is removed.**
  ([#46](https://github.com/tari-project/stable-coin/pull/46)) `transfer_fee`, `FeeSpec`,
  `set_config_transfer_fee_fixed` and `set_config_transfer_fee_percentage` are gone; the fee was stored but
  never applied. `StableCoinConfig` is now `{ max_supply }`.

### Added

- **Supply cap.** ([#45](https://github.com/tari-project/stable-coin/pull/45)) `StableCoinConfig.max_supply`
  caps the outstanding supply: `increase_supply` fails past it and `instantiate` rejects a larger initial
  supply. The governor changes it with `set_config_max_supply`, which cannot go below the current supply. In
  `issuer-no-user-badge` and `issuer` the cap counts the coin plus the wrapped token.
- **Freeze batch cap.** ([#45](https://github.com/tari-project/stable-coin/pull/45)) `freeze_utxos` and
  `unfreeze_utxos` take at most 100 UTXOs per call.
- **Threat model, view key and integration notes** in each template's README.
  ([#46](https://github.com/tari-project/stable-coin/pull/46))

### Fixed

- `pause`, `unpause`, `freeze_utxos`, `unfreeze_utxos`, `burn_utxo` and `set_user_exchange_limit` no longer
  fail in a transaction that has no main signer. ([#46](https://github.com/tari-project/stable-coin/pull/46))
- The sample manifests no longer hardcode a resource view key; they take `view_key` and `roles` as variables.
  ([#46](https://github.com/tari-project/stable-coin/pull/46))

### Build

- Each template's `Cargo.lock` is tracked. CI tests with `--locked` and checks advisories with `cargo deny`.
  ([#46](https://github.com/tari-project/stable-coin/pull/46))
