// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import { types } from "@tariproject/tarijs";
import { RejectReason, Substate, SubstateId } from "@tariproject/typescript-bindings";


export interface NewIssuerParams {
  initialSupply: string;
  tokenSymbol: string;
  tokenMetadata: any;
  viewKey: string;
  enableWrappedToken: boolean;
}

export class SimpleTransactionResult {
  private inner: types.TransactionResult;

  constructor(result: types.TransactionResult) {
    this.inner = result;
  }

  static from(result: types.TransactionResult): SimpleTransactionResult {
    return new SimpleTransactionResult(result);
  }

  public get status(): types.TransactionStatus {
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
        .map(([id, val]: [SubstateId, Substate]) => {
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

          console.log("Unknown substate type", id);
          return null;
        })
        .filter((x: [string, any] | null) => x !== null),
      down_substates: accept.down_substates
        .map(([id, _version]: [SubstateId, number]) => {
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

export type SubstateType = "Component" | "Resource" | "Vault" | "NonFungible";
