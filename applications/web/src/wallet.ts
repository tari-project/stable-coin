import {
    AccountData,
    Network,
    SimpleTransactionResult,
    SubmitTransactionRequest,
    Substate as TariJsSubstate,
    TariProvider,
    TariSigner,
    TransactionStatus
} from "@tari-project/tarijs-all";
import {NewIssuerParams, splitOnce} from "./types";
import {
    Amount,
    ComponentAddress,
    Instruction,
    KeyBranch,
    ResourceAddress,
    SubstateRequirement,
    SubstateType,
    VaultId,
} from "@tari-project/typescript-bindings";

const NETWORK = Network.Igor;

export default class TariWallet<TProvider extends TariProvider, TSigner extends TariSigner> {
    private provider: TProvider;
    private signer: TSigner;

    constructor(provider: TProvider, signer: TSigner) {
        this.provider = provider;
        this.signer = signer;
    }

    public isConnected(): boolean {
        return this.provider.isConnected();
    }

    public static new<TProvider extends TariProvider, TSigner extends TariSigner>(provider: TProvider, signer: TSigner): TariWallet<TProvider, TSigner> {
        return new TariWallet(provider, signer);
    }

    public providerName(): string {
        return this.provider.providerName;
    }

    // public async getTemplateDefinition(template_address: string) {
    //     return await this.provider.getTemplateDefinition(template_address);
    // }

    public async listSubstates(template: string | null, substateType: SubstateType | null) {
        return await this.provider.listSubstates({
            filter_by_template: template,
            filter_by_type: substateType,
            limit: 100,
            offset: 0,
        });
    }

    public async getSubstate(substateId: string): Promise<TariJsSubstate> {
        return await this.signer.getSubstate(substateId);
    }

    public async submitTransactionAndWait(request: SubmitTransactionRequest) {
        const resp = await this.signer.submitTransaction(request);
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

            if (resp.status == TransactionStatus.Rejected || resp.status == TransactionStatus.InvalidTransaction) {
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
        fee: number = 2500,
    ): Promise<SimpleTransactionResult> {
        const account = await this.signer.getAccount();

        const fee_instructions = [
            {
                CallMethod: {
                    call: {Address: account.component_address},
                    method: "pay_fee",
                    args: [`Amount(${fee})`],
                },
            },
        ];

        const instructions = [
            {
                CallFunction: {
                    address: templateAddress,
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
            {PutLastInstructionOutputOnWorkspace: {key: 0}},
            {
                CallMethod: {
                    call: {Address: account.component_address},
                    method: "deposit",
                    args: [{Workspace: {id: 0, offset: null}}],
                },
            },
            "DropAllProofsInWorkspace",
        ] as Instruction[];

        const inputs = [{substate_id: account.component_address, version: null}] as SubstateRequirement[];

        const request = {
            account_id: account.account_id,
            transaction: {
                network: NETWORK,
                fee_instructions,
                instructions,
                inputs,
                dry_run: false,
                min_epoch: null,
                max_epoch: null,
                is_seal_signer_authorized: true,
            },
            detect_inputs_use_unversioned: true
        } as SubmitTransactionRequest;

        const result = await this.submitTransactionAndWait(request);
        return SimpleTransactionResult.fromResponse(result);
    }

    public async increaseSupply(
        component_address: ComponentAddress,
        badge_resource: ResourceAddress,
        amount: number,
        fee: number = 2000,
    ): Promise<SimpleTransactionResult> {
        return await this.callRestrictedMethod(component_address, badge_resource, "increase_supply", [amount], empty, [], fee);
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
                        PutLastInstructionOutputOnWorkspace: {key: 1},
                    },
                    {
                        CallMethod: {
                            call: {Address: destAccount},
                            method: "deposit",
                            args: [{Workspace: {id: 1}}],
                        },
                    },
                ] as Instruction[],
            [],
            fee,
        );
    }

    async createUser(issuerComponent: string, adminBadgeResource: string, userId: number, userAccount: string) {
        const addBadgeToUserAccount = (_account: AccountData) => [
            {
                PutLastInstructionOutputOnWorkspace: {key: 1},
            },
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "deposit",
                    args: [{Workspace: {id: 1, offset: null}}],
                },
            },
        ] as Instruction[];

        const extraInputs = [
            {substate_id: userAccount, version: null},
        ];

        return await this.callRestrictedMethod(
            issuerComponent,
            adminBadgeResource,
            "create_new_user",
            [userId, userAccount],
            addBadgeToUserAccount,
            extraInputs,
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
                substate_id: `nft_${userBadgeResource}_u64_${userId}`,
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
                {PutLastInstructionOutputOnWorkspace: {key: 1}},
                {
                    CallMethod: {
                        call: {Address: userAccount},
                        method: "deposit",
                        args: [{Workspace: {id: 1, offset: null}}],
                    },
                },
            ] as Instruction[];

        const extraInputs = [
            {
                substate_id: `nft_${userBadgeResource}_u64_${userId}`,
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
                substate_id: `nft_${userBadgeResource}_u64_${userId}`,
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
                substate_id: `nft_${userBadgeResource}_u64_${userId}`,
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
        return await this.signer.getPublicKey(branch, index);
    }

    public async getAccount() {
        return await this.signer.getAccount();
    }

    public async getConfidentialVaultBalance(vaultId: VaultId, min: number | null = null, max: number | null = null) {
        return await this.signer.getConfidentialVaultBalances({
            vault_id: vaultId,
            minimum_expected_value: min,
            maximum_expected_value: max,
            view_key_id: 0
        });
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
        const feeAccount = await this.signer.getAccount();

        const instructions = [
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "create_proof_for_resource",
                    args: [userBadgeResource],
                },
            },
            {PutLastInstructionOutputOnWorkspace: {key: 0}},
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "withdraw",
                    args: [stableCoinResource, amount],
                },
            },
            {PutLastInstructionOutputOnWorkspace: {key: 1}},
            {
                CallMethod: {
                    call: {Address: issuerComponent},
                    method: "exchange_stable_for_wrapped_tokens",
                    args: [{Workspace: {id: 0, offset: null}}, {Workspace: {id: 1, offset: null}}],
                },
            },
            {PutLastInstructionOutputOnWorkspace: {key: 2}},
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "deposit",
                    args: [{Workspace: {id: 2, offset: null}}],
                },
            },
            "DropAllProofsInWorkspace",
        ] as Instruction[];

        const [_t, userBadgeResx] = splitOnce(userBadgeResource, '_')!;
        const required_substates = [
            {substate_id: userAccount, version: null},
            {substate_id: issuerComponent, version: null},
            {substate_id: userBadgeResource, version: null},
            {
                substate_id: `nft_${userBadgeResx}_u64_${userId}`,
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
        const feeAccount = await this.signer.getAccount();

        const instructions = [
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "create_proof_for_resource",
                    args: [userBadgeResource],
                },
            },
            {PutLastInstructionOutputOnWorkspace: {key: 0}},
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "withdraw",
                    args: [wrappedCoinResource, amount],
                },
            },
            {PutLastInstructionOutputOnWorkspace: {key: 1}},
            {
                CallMethod: {
                    call: {Address: issuerComponent},
                    method: "exchange_wrapped_for_stable_tokens",
                    args: [{Workspace: 0}, {Workspace: 1}],
                },
            },
            {PutLastInstructionOutputOnWorkspace: {key: 2}},
            {
                CallMethod: {
                    call: {Address: userAccount},
                    method: "deposit",
                    args: [{Workspace: 2}],
                },
            },
            "DropAllProofsInWorkspace",
        ] as Instruction[];

        const required_substates = [
            {substate_id: userAccount, version: null},
            {substate_id: issuerComponent, version: null},
            {substate_id: userBadgeResource, version: null},
            {
                substate_id: `nft_${userBadgeResource}_u64_${userId}`,
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
        extraInstructions: (account: AccountData) => Array<Instruction>,
        extraInputs: Array<SubstateRequirement>,
        fee: number = 2000,
    ): Promise<SimpleTransactionResult> {
        const account = await this.signer.getAccount();

        const extra = extraInstructions(account);

        const instructions = [
            {
                CallMethod: {
                    call: {Address: account.component_address},
                    method: "create_proof_for_resource",
                    args: [admin_badge_resx],
                },
            },
            {
                PutLastInstructionOutputOnWorkspace: {key: 0},
            },
            {
                CallMethod: {
                    call: {Address: component_address},
                    method,
                    args,
                },
            },
            ...extra,
            "DropAllProofsInWorkspace",
        ] as Instruction[];

        const required_substates = [
            {substate_id: account.component_address, version: null},
            {substate_id: component_address, version: null},
            {substate_id: admin_badge_resx, version: null},
            ...extraInputs,
        ] as SubstateRequirement[];

        return await this.submitTransaction(account, instructions, required_substates, fee);
    }

    async submitTransaction(
        account: AccountData,
        instructions: Instruction[],
        inputs: SubstateRequirement[],
        fee: Amount,
    ) {
        const fee_instructions = [
            {
                CallMethod: {
                    call: {Address: account.component_address},
                    method: "pay_fee",
                    args: [`Amount(${fee})`],
                },
            },
        ] as Instruction[];

        const request = {
            account_id: account.account_id,
            transaction: {
                network: NETWORK,
                fee_instructions,
                instructions,
                inputs,
                dry_run: false,
                min_epoch: null,
                max_epoch: null,
                is_seal_signer_authorized: true,
            },
            detect_inputs_use_unversioned: true
        } as SubmitTransactionRequest;

        const result = await this.submitTransactionAndWait(request);
        return SimpleTransactionResult.fromResponse(result);
    }
}

function empty<T>(): Array<T> {
    return [];
}
