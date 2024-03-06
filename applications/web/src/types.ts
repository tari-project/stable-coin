// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import * as tarijs from "@tariproject/tarijs";


export interface NewIssuerParams {
    initialSupply: string;
    tokenSymbol: string;
    tokenMetadata: object;
    enableWrappedToken: boolean;
}

export class SimpleTransactionResult {
    private inner: TransactionResult;

    constructor(result: TransactionResult) {
        this.inner = result;
    }

    static from(result: TransactionResult): SimpleTransactionResult {
        return new SimpleTransactionResult(result);
    }

    public get status(): tarijs.providers.types.TransactionStatus {
        return this.inner.status
    }

    public get transactionId(): string {
        return this.inner.transaction_id
    }

    public get result(): object | null {
        return this.inner.result
    }

    public get rejectReason(): object | null {
        if (this.rejected) {
            return this.rejected;
        }

        if (this.onlyFeeAccepted) {
            return this.onlyFeeAccepted[0];
        }

        return null;
    }

    public get accept(): SubstateDiff | null {
        const accept = this.inner.result.result.Accept;
        if (!accept) {
            return null;
        }

        return {
            up_substates: accept.up_substates.map(([id, val]: [object, object]) => {
                switch (Object.keys(id)[0]) {
                    case "Component":
                        return ["Component", id.Component, val.substate.Component];
                    case "Resource":
                        return ["Resource", id.Resource, val.substate.Resource];
                    case "Vault":
                        return ["Vault", id.Vault, val.substate.Vault];
                    case "NonFungible":
                        return ["NonFungible", id.NonFungible, val.substate.NonFungible];
                    default:
                        console.log("Unknown substate type", id);
                        return null;
                }
            }).filter((x) => x),
            down_substates: accept.down_substates.map(([id, _version]: [object, number]) => {
                switch (Object.keys(id)[0]) {
                    case "Component":
                        return ["Component", id.Component];
                    case "Resource":
                        return ["Resource", id.Resource];
                    case "Vault":
                        return ["Vault", id.Vault];
                    case "NonFungible":
                        return ["NonFungible", id.NonFungible];
                    default:
                        console.log("Unknown substate type", id);
                        return null;
                }
            }).filter((x) => x),
        };
    }

    public get onlyFeeAccepted(): [object, string] | null {
        return this.inner.result.result.AcceptFeeRejectRest;
    }

    public get rejected(): object | null {
        return this.inner.result.result.Reject;
    }

}


export interface SubstateDiff {
    up_substates: SubstateTuple[],
    down_substates: SubstateTypeAndId[]
}

export type SubstateTuple = [SubstateType, string, object];
export type SubstateTypeAndId = [SubstateType, string];

export type SubstateType = "Component" | "Resource" | "Vault" | "NonFungible";