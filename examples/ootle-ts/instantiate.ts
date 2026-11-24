// Copyright 2026 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

// Deploying and governing an `issuer-minimal` stable coin from TypeScript with ootle.ts.
//
// The coin is governed by a 1-of-2 rule: either of two keys can act alone. Roles are the
// component's access rules, so they are changed with a transaction instruction and need no
// encoding at all; only `instantiate` takes a CBOR-encoded `RoleConfig`.

import {
  AccessRuleBuilder,
  CborBuilder,
  TransactionBuilder,
  amountLiteral,
  getVaultIdsForAccount,
  intLiteral,
  metadataLiteral,
  noneLiteral,
  publicKeyLiteral,
  requirement,
  resolveMaxEpoch,
  sealTransaction,
  signTransaction,
  stringLiteral,
} from "@tari-project/ootle";
import type { Signer } from "@tari-project/ootle";
import type {
  AccessRule,
  ComponentAddress,
  PublishedTemplateAddress,
  UnsignedTransactionV1,
} from "@tari-project/ootle-ts-bindings";
import type { IndexerProvider } from "@tari-project/ootle-indexer";

const MAX_FEE = 20_000n;

/** At least `m` of the keys must sign: the Rust `rule!(m_of_n(m, public_key(a), public_key(b), ..))`. */
export const mOfN = (m: number, publicKeysHex: string[]): AccessRule =>
  AccessRuleBuilder.mOfN(m, ...publicKeysHex.map(requirement.publicKey)).build();

export interface Deployer {
  provider: IndexerProvider;
  network: number;
  /** Signs the transaction. */
  signer: Signer;
  /** The signer's account, which pays the fee. */
  account: ComponentAddress;
}

export interface CoinParams {
  template: PublishedTemplateAddress;
  /** Either governor key can act alone. */
  governorKeysHex: [string, string];
  /** The coin's view key. It reveals every amount and cannot be rotated. */
  viewPublicKeyHex: string;
}

/** Instantiates the coin. The new `component_…` and `resource_…` are in the receipt's diff. */
export async function instantiateStableCoin(deployer: Deployer, params: CoinParams) {
  // `RoleConfig` is a struct. Its fields are set by their `#[n(i)]` index in the template:
  //   0 governor, 1 minter, 2 burner, 3 treasurer, 4 compliance, 6 pauser
  // Only the governor is set, so every role stays with the governor. To give a role its own
  // rule at deployment, add e.g. `.field(1, AccessRuleBuilder.publicKey(minterKeyHex))`.
  const roles = CborBuilder.struct()
    .field(0, AccessRuleBuilder.mOfN(1, ...params.governorKeysHex.map(requirement.publicKey)))
    .finish();

  const builder = (await newBuilder(deployer))
    .allocateAddress("Component", "coin")
    .callFunction({ templateAddress: params.template, functionName: "instantiate" }, [
      { Workspace: "coin" }, // address_alloc
      amountLiteral(1_000_000_000n), // initial_token_supply
      stringLiteral("STC"), // token_symbol (at most 8 characters)
      metadataLiteral({ provider_name: "Stable Coin Inc." }), // token_metadata
      intLiteral(2n), // divisibility
      publicKeyLiteral(params.viewPublicKeyHex), // view_key
      noneLiteral(), // config: None leaves the supply uncapped
      roles,
    ]);
  return submit(deployer, builder.buildUnsignedTransaction());
}

/**
 * Gives the minter and pauser roles to their own keys. Sign with a governor key.
 *
 * The instruction replaces every method rule at once: list each role that is not left with the
 * governor, with all of its methods. A method without a rule can only be called by the governor.
 */
export async function setRoles(
  deployer: Deployer,
  coin: ComponentAddress,
  minterKeyHex: string,
  pauserKeyHex: string,
) {
  const builder = (await newBuilder(deployer))
    .addInput({ substate_id: coin, version: null })
    .setComponentAccessRules(coin, {
      method_access: {
        increase_supply: AccessRuleBuilder.publicKey(minterKeyHex).build(),
        pause: AccessRuleBuilder.publicKey(pauserKeyHex).build(),
      },
      default: "DenyAll",
    });
  return submit(deployer, builder.buildUnsignedTransaction());
}

/**
 * Hands the coin to a new governor rule. Sign with a current governor key.
 *
 * This is immediate and nothing checks that anyone holds the new rule: a mistyped key leaves the
 * coin without a governor for good.
 */
export async function setGovernor(deployer: Deployer, coin: ComponentAddress, governor: AccessRule) {
  const builder = (await newBuilder(deployer))
    .addInput({ substate_id: coin, version: null })
    .setComponentOwnerRule(coin, { ByAccessRule: governor });
  return submit(deployer, builder.buildUnsignedTransaction());
}

async function newBuilder({ provider, network, account }: Deployer): Promise<TransactionBuilder> {
  const vaults = await getVaultIdsForAccount(provider, account);
  return new TransactionBuilder(network, await resolveMaxEpoch(provider))
    .withInputs([{ substate_id: account, version: null }, ...vaults.map((v) => ({ substate_id: v, version: null }))])
    .feeTransactionPayFromComponent(account, MAX_FEE);
}

async function submit({ provider, signer }: Deployer, unsigned: UnsignedTransactionV1) {
  const signed = await signTransaction([signer], unsigned);
  const { transaction_id } = await provider.submitTransaction(sealTransaction(signed));
  const pending = await provider.watchTransactionSSE(transaction_id);
  await pending.watch();
  return pending.getReceipt();
}
