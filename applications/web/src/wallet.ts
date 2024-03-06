import {providers} from '@tariproject/tarijs';
import {TariProvider} from "@tariproject/tarijs/dist/providers";
import {Account} from "@tariproject/tarijs/dist/providers/types";
import {NewIssuerParams, SimpleTransactionResult} from "./types.ts";
import {ComponentAddress, Instruction, ResourceAddress} from "@tariproject/typescript-bindings";

const {
    TariProvider,
    metamask: {MetamaskTariProvider},
    walletDaemon: {WalletDaemonTariProvider},
    types: {
        SubstateRequirement,
        TransactionSubmitRequest,
        TransactionStatus
    }
} = providers;

export default class TariWallet<TProvider extends TariProvider> {
    private provider: TProvider;

    constructor(provider: TProvider) {
        this.provider = provider;
    }


    public isConnected(): boolean {
        return this.provider.isConnected();
    }

    public static new<TProvider>(provider: TProvider): TariWallet<TProvider> {
        return new TariWallet(provider);
    }

    public async getTemplateDefinition(template_address: string) {
        const resp = await this.provider.getTemplateDefinition(template_address);
        return resp.template_definition;
    }

    public async listSubstates(template: string | null, substateType: SubstateType | null) {
        if (this.provider.providerName !== "WalletDaemon") {
            throw new Error(`Unsupported provider ${this.provider.providerName}`);
        }
        const substates = await (this.provider as WalletDaemonTariProvider).listSubstates(
            template,
            substateType
        );
        return substates;
    }

    public async createFreeTestCoins() {
        console.log("createFreeTestCoins", this.provider.providerName);
        switch (this.provider.providerName) {
            case "WalletDaemon":
                const walletProvider = this.provider as WalletDaemonTariProvider;
                await walletProvider.createFreeTestCoins();
                break;
            case "Metamask":
                const metamaskProvider = this.provider as MetamaskTariProvider;
                await metamaskProvider.createFreeTestCoins(0);
                break;
            default:
                throw new Error(`Unsupported provider: ${this.provider.providerName}`);
        }
    }

    public async getSubstate(substateId: string) {
        const resp = await this.provider.getSubstate(substateId);
        return resp;
    }

    public async submitTransactionAndWait(
        request: SubmitTransactionRequest,
    ) {
        const resp = await this.provider.submitTransaction(request);
        let result = await this.waitForTransactionResult(resp.transaction_id);
        return result;
    }

    public async waitForTransactionResult(transactionId: string) {
        while (true) {
            const resp = await this.provider.getTransactionResult(transactionId);
            const FINALIZED_STATUSES = [
                TransactionStatus.Accepted,
                TransactionStatus.Rejected,
                TransactionStatus.InvalidTransaction,
                TransactionStatus.OnlyFeeAccepted,
                TransactionStatus.DryRun
            ];

            if (resp.status == TransactionStatus.Rejected) {
                throw new Error(`Transaction rejected: ${JSON.stringify(resp.result)}`);
            }

            if (FINALIZED_STATUSES.includes(resp.status)) {
                return resp;
            }
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
    }

    public async createNewIssuer(
        templateAddress: string,
        params: NewIssuerParams,
        fee: number = 2000
    ): Promise<SimpleTransactionResult> {
        const account = await this.provider.getAccount();

        const fee_instructions = [
            {
                CallMethod: {
                    component_address: account.address,
                    method: "pay_fee",
                    args: [`Amount(${fee})`]
                }
            }
        ];

        const instructions = [
            {
                CallFunction: {
                    template_address: templateAddress,
                    function: "instantiate",
                    args: [
                        params.initialSupply,
                        params.tokenSymbol,
                        Object.keys(params.tokenMetadata).map((k) => `${k}=${params.tokenMetadata[k]}`).join(','),
                        params.enableWrappedToken ? "true" : "false"
                    ]
                }
            },
            {PutLastInstructionOutputOnWorkspace: {key: [0]}},
            {
                CallMethod: {
                    component_address: account.address,
                    method: "deposit",
                    args: [{Workspace: [0]}]
                }
            },
            "DropAllProofsInWorkspace"
        ];

        const required_substates = [
            {substate_id: account.address, version: null}
        ];

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
            max_epoch: null
        };

        const result = await this.submitTransactionAndWait(request);
        return SimpleTransactionResult.from(result);
    }

    public increaseSupply(component_address: ComponentAddress, badge_resource: ResourceAddress, amount: number, fee: number = 2000) {
        return this.callRestrictedMethod(component_address, badge_resource, "increase_supply", [amount], empty, fee)
    }

    public decreaseSupply(component_address: ComponentAddress, badge_resource: ResourceAddress, amount: number, fee: number = 2000) {
        return this.callRestrictedMethod(component_address, badge_resource, "decrease_supply", [amount], empty, fee)
    }

    public transfer(issuerComponent: ComponentAddress, badgeResource: ResourceAddress, destAccount: string, amount: number, fee: number = 2000) {
        return this.callRestrictedMethod(issuerComponent, badgeResource, "withdraw", [amount], () => [
                {
                    PutLastInstructionOutputOnWorkspace: {key: [1]}
                },
                {
                    CallMethod: {
                        component_address: destAccount,
                        method: "deposit",
                        args: [{Workspace: [1]}]
                    }
                }
            ] as Instruction[],
            fee
        )
    }

    async callRestrictedMethod(component_address: ComponentAddress, badge_resource: ResourceAddress, method: string, args: Array<any>, extraInstructions: (account: Account) => Array<Instruction>, fee: number = 2000) {
        const account = await this.provider.getAccount();

        const fee_instructions = [
            {
                CallMethod: {
                    component_address: account.address,
                    method: "pay_fee",
                    args: [`Amount(${fee})`]
                }
            }
        ];

        const extra = extraInstructions(account);

        const instructions = [
            {
                CallMethod: {
                    component_address: account.address,
                    method: "create_proof_for_resource",
                    args: [badge_resource]
                }
            },
            {
                PutLastInstructionOutputOnWorkspace: {key: [0]}
            },
            {
                CallMethod: {
                    component_address,
                    method,
                    args,
                }
            },
            ...extra,
            "DropAllProofsInWorkspace"
        ];

        const required_substates = [
            {substate_id: account.address, version: null},
            {substate_id: component_address, version: null},
            {substate_id: badge_resource, version: null}
        ];

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
            max_epoch: null
        };

        const result = await this.submitTransactionAndWait(request);
        return SimpleTransactionResult.from(result);
    }

    async createUser(issuerComponent: string, adminBadgeResource: string, userId: number, userAccount: string) {
        const addBadgeToUserAccount = (_account: Account) => [
            {
                PutLastInstructionOutputOnWorkspace: {key: [1]}
            },
            {
                CallMethod: {
                    component_address: userAccount,
                    method: "deposit",
                    args: [{Workspace: [1]}]
                }
            }
        ];

        return await this.callRestrictedMethod(issuerComponent, adminBadgeResource, "create_new_user", [userId, userAccount], addBadgeToUserAccount, 2000);
    }

    public async revokeUserAccess(
        issuerComponent: ComponentAddress,
        adminBadgeResource: ResourceAddress,
        userId: number,
        vaultId: string,
        fee: number = 2000
    ): Promise<SimpleTransactionResult> {
        return await this.callRestrictedMethod(issuerComponent, adminBadgeResource, "blacklist_user", [vaultId, userId], [], fee);
    }


}


function empty<T>(): Array<T> {
    return [];
}