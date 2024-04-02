import { TariProvider, MetamaskTariProvider, WalletDaemonTariProvider } from "@tariproject/tarijs";
import {
  Account,
  SubmitTransactionRequest,
  TransactionStatus,
  SubstateRequirement,
} from "@tariproject/tarijs";
import { NewIssuerParams, SimpleTransactionResult } from "./types.ts";
import {
  ComponentAddress,
  Instruction,
  ResourceAddress,
  VaultId,
  Amount,
  SubstateType, Substate,
} from "@tariproject/typescript-bindings";
import { KeyBranch } from "@tariproject/typescript-bindings/wallet-daemon-client.ts";


export default class TariWallet<TProvider extends TariProvider> {
  private provider: TProvider;

  constructor(provider: TProvider) {
    this.provider = provider;
  }

  public isConnected(): boolean {
    return this.provider.isConnected();
  }

  public static new<TProvider extends TariProvider>(provider: TProvider): TariWallet<TProvider> {
    return new TariWallet(provider);
  }

  public async getTemplateDefinition(template_address: string) {
    return await this.provider.getTemplateDefinition(template_address);
  }

  public async listSubstates(template: string | null, substateType: SubstateType | null) {
    if (this.provider.providerName !== "WalletDaemon") {
      throw new Error(`Unsupported provider ${this.provider.providerName}`);
    }
    const substates = await (this.provider as unknown as WalletDaemonTariProvider).listSubstates(template, substateType);
    return substates;
  }

  public async createFreeTestCoins() {
    console.log("createFreeTestCoins", this.provider.providerName);
    switch (this.provider.providerName) {
      case "WalletDaemon":
        const walletProvider = this.provider as unknown as WalletDaemonTariProvider;
        await walletProvider.createFreeTestCoins();
        break;
      case "Metamask":
        const metamaskProvider = this.provider as unknown as MetamaskTariProvider;
        await metamaskProvider.createFreeTestCoins(0);
        break;
      default:
        throw new Error(`Unsupported provider: ${this.provider.providerName}`);
    }
  }

  public async getSubstate(substateId: string): Promise<{ value: Substate }> {
    const resp = await this.provider.getSubstate(substateId);
    return resp as { value: Substate };
  }

  public async submitTransactionAndWait(request: SubmitTransactionRequest) {
    const resp = await this.provider.submitTransaction(request);
    return await this.waitForTransactionResult(resp.transaction_id);
  }

  public async waitForTransactionResult(transactionId: string) {
    while (true) {
      const resp = await this.provider.getTransactionResult(transactionId);
      const FINALIZED_STATUSES = [
        TransactionStatus.Accepted,
        TransactionStatus.Rejected,
        TransactionStatus.InvalidTransaction,
        TransactionStatus.OnlyFeeAccepted,
        TransactionStatus.DryRun,
      ];

      if (resp.status == TransactionStatus.Rejected) {
        throw new Error(`Transaction rejected: ${JSON.stringify(resp.result)}`);
      }

      if (FINALIZED_STATUSES.includes(resp.status)) {
        return resp;
      }
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }

  public async createNewIssuer(
    templateAddress: string,
    params: NewIssuerParams,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const account = await this.provider.getAccount();

    const fee_instructions = [
      {
        CallMethod: {
          component_address: account.address,
          method: "pay_fee",
          args: [`Amount(${fee})`],
        },
      },
    ];

    const instructions = [
      {
        CallFunction: {
          template_address: templateAddress,
          function: "instantiate",
          args: [
            params.initialSupply,
            params.tokenSymbol,
            Object.keys(params.tokenMetadata)
              .map((k) => `${k}=${params.tokenMetadata[k as keyof object]}`)
              .join(","),
            params.viewKey,
            params.enableWrappedToken ? "true" : "false",
          ],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [0] } },
      {
        CallMethod: {
          component_address: account.address,
          method: "deposit",
          args: [{ Workspace: [0] }],
        },
      },
      "DropAllProofsInWorkspace",
    ] as Instruction[];

    const required_substates = [{ substate_id: account.address, version: null }] as SubstateRequirement[];

    const request = {
      account_id: account.account_id,
      fee_instructions,
      instructions,
      inputs: [],
      input_refs: [],
      override_inputs: false,
      required_substates,
      is_dry_run: false,
      min_epoch: null,
      max_epoch: null,
    } as SubmitTransactionRequest;

    const result = await this.submitTransactionAndWait(request);
    return SimpleTransactionResult.from(result);
  }

  public increaseSupply(
    component_address: ComponentAddress,
    badge_resource: ResourceAddress,
    amount: number,
    fee: number = 2000,
  ) {
    return this.callRestrictedMethod(component_address, badge_resource, "increase_supply", [amount], empty, [], fee);
  }

  public decreaseSupply(
    component_address: ComponentAddress,
    badge_resource: ResourceAddress,
    amount: number,
    fee: number = 2000,
  ) {
    return this.callRestrictedMethod(component_address, badge_resource, "decrease_supply", [amount], empty, [], fee);
  }

  public transfer(
    issuerComponent: ComponentAddress,
    badgeResource: ResourceAddress,
    destAccount: string,
    amount: number,
    fee: number = 2000,
  ) {
    return this.callRestrictedMethod(
      issuerComponent,
      badgeResource,
      "withdraw",
      [amount],
      () =>
        [
          {
            PutLastInstructionOutputOnWorkspace: { key: [1] },
          },
          {
            CallMethod: {
              component_address: destAccount,
              method: "deposit",
              args: [{ Workspace: [1] }],
            },
          },
        ] as Instruction[],
      [],
      fee,
    );
  }

  async createUser(issuerComponent: string, adminBadgeResource: string, userId: number, userAccount: string) {
    const addBadgeToUserAccount = (_account: Account) => [
      {
        PutLastInstructionOutputOnWorkspace: { key: [1] },
      },
      {
        CallMethod: {
          component_address: userAccount,
          method: "deposit",
          args: [{ Workspace: [1] }],
        },
      },
    ] as Instruction[];

    return await this.callRestrictedMethod(
      issuerComponent,
      adminBadgeResource,
      "create_new_user",
      [userId, userAccount],
      addBadgeToUserAccount,
      [],
    );
  }

  public async revokeUserAccess(
    issuerComponent: ComponentAddress,
    adminBadgeResource: ResourceAddress,
    userBadgeResource: ResourceAddress,
    userId: number,
    vaultId: VaultId,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const extraInputs = [
      {
        substate_id: `${userBadgeResource} nft_u64:${userId}`,
        version: null,
      },
    ] as SubstateRequirement[];
    return await this.callRestrictedMethod(
      issuerComponent,
      adminBadgeResource,
      "blacklist_user",
      [vaultId, userId],
      empty,
      extraInputs,
      fee,
    );
  }

  public async reinstateUserAccess(
    issuerComponent: ComponentAddress,
    adminBadgeResource: ResourceAddress,
    userBadgeResource: ResourceAddress,
    userId: number,
    userAccount: ComponentAddress,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const extra = () =>
      [
        { PutLastInstructionOutputOnWorkspace: { key: [1] } },
        {
          CallMethod: {
            component_address: userAccount,
            method: "deposit",
            args: [{ Workspace: [1] }],
          },
        },
      ] as Instruction[];

    const extraInputs = [
      {
        substate_id: `${userBadgeResource} nft_u64:${userId}`,
        version: null,
      },
    ] as SubstateRequirement[];

    return await this.callRestrictedMethod(
      issuerComponent,
      adminBadgeResource,
      "remove_from_blacklist",
      [userId],
      extra,
      extraInputs,
      fee,
    );
  }

  public async setUserExchangeLimit(
    issuerComponent: ComponentAddress,
    adminBadgeResource: ResourceAddress,
    userBadgeResource: ResourceAddress,
    userId: number,
    newLimit: number,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const extraInputs = [
      {
        substate_id: `${userBadgeResource} nft_u64:${userId}`,
        version: null,
      },
    ] as SubstateRequirement[];

    return await this.callRestrictedMethod(
      issuerComponent,
      adminBadgeResource,
      "set_user_wrapped_exchange_limit",
      [userId, newLimit],
      empty,
      extraInputs,
      fee,
    );
  }

  public async recallTokens(
    issuerComponent: ComponentAddress,
    adminBadgeResource: ResourceAddress,
    userAccount: ComponentAddress,
    userBadgeResource: ResourceAddress,
    userId: number,
    amount: Amount,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const extraInputs = [
      {
        substate_id: `${userBadgeResource} nft_u64:${userId}`,
        version: null,
      },
      {
        substate_id: userAccount,
        version: null,
      },
    ] as SubstateRequirement[];

    return await this.callRestrictedMethod(
      issuerComponent,
      adminBadgeResource,
      "recall_tokens",
      [userId, [], amount],
      empty,
      extraInputs,
      fee,
    );
  }

  public async getPublicKey(branch: KeyBranch, index: number): Promise<string> {
    return await this.provider.getPublicKey(branch, index);
  }

  public async getAccount() {
    return await this.provider.getAccount();
  }

  public async getConfidentialVaultBalance(vaultId: VaultId, min: number | null = null, max: number | null = null) {
    return await this.provider.getConfidentialVaultBalances(0, vaultId, min, max);
  }

  public async exchangeStableForWrappedToken(
    issuerComponent: ComponentAddress,
    userAccount: ComponentAddress,
    stableCoinResource: ResourceAddress,
    userBadgeResource: ResourceAddress,
    userId: number,
    amount: Amount,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const feeAccount = await this.provider.getAccount();

    const instructions = [
      {
        CallMethod: {
          component_address: userAccount,
          method: "create_proof_for_resource",
          args: [userBadgeResource],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [0] } },
      {
        CallMethod: {
          component_address: userAccount,
          method: "withdraw",
          args: [stableCoinResource, amount],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [1] } },
      {
        CallMethod: {
          component_address: issuerComponent,
          method: "exchange_stable_for_wrapped_tokens",
          args: [{ Workspace: [0] }, { Workspace: [1] }],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [2] } },
      {
        CallMethod: {
          component_address: userAccount,
          method: "deposit",
          args: [{ Workspace: [2] }],
        },
      },
      "DropAllProofsInWorkspace",
    ] as Instruction[];

    const required_substates = [
      { substate_id: userAccount, version: null },
      { substate_id: issuerComponent, version: null },
      { substate_id: userBadgeResource, version: null },
      {
        substate_id: `${userBadgeResource} nft_u64:${userId}`,
        version: null,
      },
      {
        substate_id: stableCoinResource,
        version: null,
      },
    ] as SubstateRequirement[];

    return await this.submitTransaction(feeAccount, instructions, required_substates, fee);
  }

  public async exchangeWrappedForStable(
    issuerComponent: ComponentAddress,
    userAccount: ComponentAddress,
    wrappedCoinResource: ResourceAddress,
    userBadgeResource: ResourceAddress,
    userId: number,
    amount: Amount,
    fee: number = 2000,
  ): Promise<SimpleTransactionResult> {
    const feeAccount = await this.provider.getAccount();

    const instructions = [
      {
        CallMethod: {
          component_address: userAccount,
          method: "create_proof_for_resource",
          args: [userBadgeResource],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [0] } },
      {
        CallMethod: {
          component_address: userAccount,
          method: "withdraw",
          args: [wrappedCoinResource, amount],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [1] } },
      {
        CallMethod: {
          component_address: issuerComponent,
          method: "exchange_wrapped_for_stable_tokens",
          args: [{ Workspace: [0] }, { Workspace: [1] }],
        },
      },
      { PutLastInstructionOutputOnWorkspace: { key: [2] } },
      {
        CallMethod: {
          component_address: userAccount,
          method: "deposit",
          args: [{ Workspace: [2] }],
        },
      },
      "DropAllProofsInWorkspace",
    ] as Instruction[];

    const required_substates = [
      { substate_id: userAccount, version: null },
      { substate_id: issuerComponent, version: null },
      { substate_id: userBadgeResource, version: null },
      {
        substate_id: `${userBadgeResource} nft_u64:${userId}`,
        version: null,
      },
      {
        substate_id: wrappedCoinResource,
        version: null,
      },
    ] as SubstateRequirement[];

    return await this.submitTransaction(feeAccount, instructions, required_substates, fee);
  }

  async callRestrictedMethod(
    component_address: ComponentAddress,
    admin_badge_resx: ResourceAddress,
    method: string,
    args: Array<any>,
    extraInstructions: (account: Account) => Array<Instruction>,
    extraInputs: Array<SubstateRequirement>,
    fee: number = 2000,
  ) {
    const account = await this.provider.getAccount();

    const extra = extraInstructions(account);

    const instructions = [
      {
        CallMethod: {
          component_address: account.address,
          method: "create_proof_for_resource",
          args: [admin_badge_resx],
        },
      },
      {
        PutLastInstructionOutputOnWorkspace: { key: [0] },
      },
      {
        CallMethod: {
          component_address,
          method,
          args,
        },
      },
      ...extra,
      "DropAllProofsInWorkspace",
    ] as Instruction[];

    const required_substates = [
      { substate_id: account.address, version: null },
      { substate_id: component_address, version: null },
      { substate_id: admin_badge_resx, version: null },
      ...extraInputs,
    ] as SubstateRequirement[];

    return await this.submitTransaction(account, instructions, required_substates, fee);
  }

  async submitTransaction(
    account: Account,
    instructions: Instruction[],
    required_substates: SubstateRequirement[],
    fee: Amount,
  ) {
    const fee_instructions = [
      {
        CallMethod: {
          component_address: account.address,
          method: "pay_fee",
          args: [`Amount(${fee})`],
        },
      },
    ] as Instruction[];

    const request = {
      account_id: account.account_id,
      fee_instructions,
      instructions,
      inputs: [],
      input_refs: [],
      override_inputs: false,
      required_substates,
      is_dry_run: false,
      min_epoch: null,
      max_epoch: null,
    } as SubmitTransactionRequest;

    const result = await this.submitTransactionAndWait(request);
    return SimpleTransactionResult.from(result);
  }
}

function empty<T>(): Array<T> {
  return [];
}
