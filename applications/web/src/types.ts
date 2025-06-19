// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import {GetTransactionResultResponse, TransactionStatus} from "@tari-project/tarijs-all";
import {RejectReason, Substate, SubstateId, SubstateType,} from "@tari-project/typescript-bindings";


export interface NewIssuerParams {
    initialSupply: string;
    tokenSymbol: string;
    tokenMetadata: any;
    viewKey: string;
    enableWrappedToken: boolean;
}

export class SimpleTransactionResult {
    private inner: GetTransactionResultResponse;

    constructor(result: GetTransactionResultResponse) {
        this.inner = result;
    }

    static from(result: GetTransactionResultResponse): SimpleTransactionResult {
        return new SimpleTransactionResult(result);
    }

    public get status(): TransactionStatus {
        return this.inner.status;
    }

    public get transactionId(): string {
        return this.inner.transaction_id;
    }

    public get result(): object | null {
        return this.inner.result;
    }

    public get rejectReason(): RejectReason | null {
        if (this.rejected) {
            return this.rejected;
        }

        if (this.onlyFeeAccepted) {
            return this.onlyFeeAccepted[1];
        }

        return null;
    }

    public get accept(): SubstateDiff | null {
        const result = this.inner.result as any;
        const accept = result?.result.Accept;
        if (!accept) {
            return null;
        }

        return {
            up_substates: accept.up_substates
                .map(([id, val]: [SubstateId | string, Substate]) => {

                    if (!val.substate) {
                        console.error("Substate is missing in the accept result", id, val);
                    }
                    if (typeof id === "string") {
                        const s = splitOnce(id, "_");
                        if (!s) {
                            throw new Error("Invalid substate ID format " + id);
                        }
                        const [type, idValue] = s;
                        if (!type || !idValue) {
                            console.log("Invalid substate ID format", id);
                            return null;
                        }
                        switch (type) {
                            case "component":
                                return ["Component", idValue, val.version, val.substate.Component];
                            case "resource":
                                return ["Resource", idValue, val.version, val.substate.Resource];
                            case "vault":
                                return ["Vault", idValue, val.version, val.substate.Vault];
                            case "nft":
                                return ["NonFungible", idValue, val.version, val.substate.NonFungible];
                            case 'txreceipt':
                                return ["TransactionReceipt", idValue, val.version, val.substate.TransactionReceipt];
                            case 'vnfp':
                                return ["ValidatorFeePool", idValue, val.version, val.substate.ValidatorFeePool];
                            case 'template':
                                return ["Template", idValue, val.version, val.substate.Template];

                            default:
                                console.log("Unknown substate type", id);
                                return null;
                        }
                    }
                    if (!id || typeof id !== "object") {
                        console.log("Invalid substate ID type", id);
                        return null;
                    }


                    console.log("Substate ID", id, val);

                    if ("Component" in id && "Component" in val.substate) {
                        return ["Component", id.Component, val.version, val.substate.Component];
                    }
                    if ("Resource" in id && "Resource" in val.substate) {
                        return ["Resource", id.Resource, val.version, val.substate.Resource];
                    }
                    if ("Vault" in id && "Vault" in val.substate) {
                        return ["Vault", id.Vault, val.version, val.substate.Vault];
                    }
                    if ("NonFungible" in id && "NonFungible" in val.substate) {
                        return ["NonFungible", id.NonFungible, val.version, val.substate.NonFungible];
                    }
                    if ("TransactionReceipt" in id && "TransactionReceipt" in val.substate) {
                        return ["TransactionReceipt", id.TransactionReceipt, val.version, val.substate.TransactionReceipt];
                    }
                    if ("ValidatorFeePool" in id && "ValidatorFeePool" in val.substate) {
                        return ["ValidatorFeePool", id.ValidatorFeePool, val.version, val.substate.ValidatorFeePool];
                    }

                    console.log("Unknown substate type", id);
                    return null;
                })
                .filter((x: [string, any] | null) => x !== null),
            down_substates: accept.down_substates
                .map(([id, _version]: [SubstateId | string, number]) => {
                    if (typeof id === "string") {
                        const s = splitOnce(id, "_");
                        if (!s) {
                            throw new Error("Invalid substate ID format " + id);
                        }
                        const [type, idValue] = s;
                        if (!type || !idValue) {
                            console.log("Invalid substate ID format", id);
                            return null;
                        }
                        switch (type) {
                            case "component":
                                return ["Component", idValue];
                            case "resource":
                                return ["Resource", idValue];
                            case "vault":
                                return ["Vault", idValue];
                            case "nft":
                                return ["NonFungible", idValue];
                            case 'txreceipt':
                                return ["TransactionReceipt", idValue];
                            case 'vnfp':
                                return ["ValidatorFeePool", idValue];
                            case 'template':
                                return ["Template", idValue];

                            default:
                                console.log("Unknown substate type", id);
                                return null;
                        }
                    }

                    if (!id || typeof id !== "object") {
                        console.log("Invalid substate ID type", id);
                        return null;
                    }

                    if ("Component" in id) {
                        return ["Component", id.Component];
                    }
                    if ("Resource" in id) {
                        return ["Resource", id.Resource];
                    }
                    if ("Vault" in id) {
                        return ["Vault", id.Vault];
                    }
                    if ("NonFungible" in id) {
                        return ["NonFungible", id.NonFungible];
                    }
                    if ("TransactionReceipt" in id) {
                        return ["TransactionReceipt", id.TransactionReceipt];
                    }
                    if ("ValidatorFeePool" in id) {
                        return ["ValidatorFeePool", id.ValidatorFeePool];
                    }

                    console.log("Unknown substate type", id);
                    return null;
                })
                .filter((x: [string, any] | null) => x !== null),
        };
    }

    public get onlyFeeAccepted(): [SubstateDiff, RejectReason] | null {
        const result = this.inner.result as any;
        return result?.result.AcceptFeeRejectRest;
    }

    public get rejected(): RejectReason | null {
        const result = this.inner.result as any;
        return result?.result.Reject;
    }
}

export function splitOnce(str: string, separator: string): [string, string] | null {
    const index = str.indexOf(separator);
    if (index === -1) {
        return null;
    }
    return [str.slice(0, index), str.slice(index + separator.length)];
}

export interface SubstateDiff {
    up_substates: UpTuple[];
    down_substates: DownTuple[];
}

export type UpTuple = [SubstateType, string, number, any];
export type DownTuple = [SubstateType, string];
