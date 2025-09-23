// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

export interface NewIssuerParams {
    initialSupply: string;
    tokenSymbol: string;
    tokenMetadata: any;
    viewKey: string;
    enableWrappedToken: boolean;
}

export function splitOnce(str: string, separator: string): [string, string] | null {
    const index = str.indexOf(separator);
    if (index === -1) {
        return null;
    }
    return [str.slice(0, index), str.slice(index + separator.length)];
}
