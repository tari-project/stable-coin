// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause


export function getValueByPath(cborRepr: object, path: string): any {
    let value = cborRepr;
    for (const part of path.split('.')) {
        if (part == "$") {
            continue;
        }
        if (value.Map) {
            value = value.Map.find((v) => v[0].Text === part)?.[1];
            if (!value) {
                return null;
            }
            continue;
        }

        if (value.Array) {
            value = value.Array[parseInt(part)];
            continue;
        }

        return null;
    }
    return maybeProcessTag(value);
}

function maybeProcessTag(maybeTag: any): any {
    if (maybeTag.Tag) {
        return processTag(maybeTag);
    }
    return maybeTag;
}

function processTag(item: any): any {
    return convertTaggedValueToString(item.Tag[0], item.Tag[1].Bytes);
    // switch (item.Tag?.[0]) {
    //     case BinaryTag.VaultId:
    //         if (!item.Tag[1]?.Bytes) {
    //             return item;
    //         }
    //         return bytesToAddressString("vault", item.Tag[1].Bytes);
    //     case BinaryTag.ComponentAddress:
    //         if (!item.Tag[1]?.Bytes) {
    //             return item;
    //         }
    //         return bytesToAddressString("component", item.Tag[1].Bytes);
    //     case BinaryTag.ResourceAddress:
    //         if (!item.Tag[1]?.Bytes) {
    //             return item;
    //         }
    //         return bytesToAddressString("resource", item.Tag[1].Bytes);
    //     default:
    //         return item;
    // }
}

function bytesToAddressString(type: String, tag: ArrayLike<unknown>): string {
    const hex = Array.from(tag, function (byte) {
        return ('0' + (byte & 0xFF).toString(16)).slice(-2);
    }).join('');

    return `${type}_${hex}`;
}

export function convertTaggedValueToString(tag: number, value: any): string | any {
    switch (tag) {
        case BinaryTag.VaultId:
            return bytesToAddressString("vault", value);
        case BinaryTag.ComponentAddress:
            return bytesToAddressString("component", value);
        case BinaryTag.ResourceAddress:
            return bytesToAddressString("resource", value);
        default:
            return value;
    }
}


enum BinaryTag {
    ComponentAddress = 128,
    Metadata = 129,
    NonFungibleAddress = 130,
    ResourceAddress = 131,
    VaultId = 132,
    TransactionReceipt = 134,
    FeeClaim = 135,
}