# ootle.ts example

[`instantiate.ts`](instantiate.ts) deploys and governs a coin from the
[`issuer-minimal`](../../templates/private_stable_coin/issuer-minimal/README.md) template with
[ootle.ts](https://github.com/tari-project/ootle.ts):

- `instantiateStableCoin` deploys a coin governed by a 1-of-2 rule: either of two keys can act alone.
- `setRoles` gives the minter and pauser roles to their own keys.
- `setGovernor` hands the coin to a new governor rule.

Only `instantiate` needs an encoded argument, the `RoleConfig`. Role changes and the governor handover are
transaction instructions that take rules as plain objects.

## Requirements

- `@tari-project/ootle` 0.20.1 or later, for `accessRuleLiteral`, `publicKeyAccessRule`, `listLiteral` and
  `noneLiteral`.
- A network running Tari Ootle 0.55 or later.

## Type-check

```bash
npm install
npm run build
```

The file exports functions and has no entry point: call them from your own code with a provider, a signer and
the signer's account.
