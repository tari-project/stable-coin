//  Copyright 2022. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

import "./Style.css";
import Grid from "@mui/material/Grid";
import * as React from "react";
import useTariProvider from "../../store/provider.ts";
import {Alert, TextField} from "@mui/material";
import Button from "@mui/material/Button";
import {ComponentAddress, ResourceAddress} from "@tariproject/typescript-bindings";
import {convertTaggedValueToString} from "../../cbor.ts";
import * as cbor from "../../cbor.ts";
import {SimpleTransactionResult, splitOnce} from "../../types.ts";
import {BoxHeading2} from "../../components/StyledComponents.ts";

interface Props {
    issuerId: ComponentAddress
    userId: number,
    userBadge: ResourceAddress,
    adminAuthBadge: ResourceAddress,
    badgeData: object,
    badgeMutableData: object,
    onChange?: (result: SimpleTransactionResult) => void,
}

function ManageUser(props: Props) {
    const {provider} = useTariProvider();

    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);
    const [success, setSuccess] = React.useState<string | null>(null);
    const [wrappedExchangeLimit, setWrappedExchangeLimit] = React.useState<number>(props.badgeMutableData.wrapped_exchange_limit);

    const [tag, v] = props.badgeData.user_account!['@@TAGGED@@'];
    const userAccount = convertTaggedValueToString(tag, v);
    const [userBadgeResource, _nft] = splitOnce(props.userBadge, ' ')!;

    const handleOnRevoke = async () => {
        setIsBusy(true);
        setSuccess(null);

        try {
            const substate = await provider.getSubstate(userAccount);
            const vaults = cbor.getValueByPath(substate.value.substate.Component.body.state, "$.vaults");
            const vaultToRevoke = vaults[userBadgeResource];
            if (!vaultToRevoke) {
                throw new Error(`User does not have a stable coin user badge ${userBadgeResource}`)
            }

            const result = await provider.revokeUserAccess(
                props.issuerId,
                props.adminAuthBadge,
                userBadgeResource,
                props.userId,
                vaultToRevoke,
            );
            if (result.accept) {
                setSuccess(`User permission revoked in transaction ${result.transactionId}`);
            } else {
                setError(new Error(`Transaction failed ${JSON.stringify(result.rejectReason)}`));
            }
            props.onChange?.(result);
        } catch (e) {
            setError(e);
        } finally {
            setIsBusy(false);
        }
    };

    const handleOnReinstate = async () => {
        setIsBusy(true);
        setSuccess(null);

        try {
            const result = await provider.reinstateUserAccess(
                props.issuerId,
                props.adminAuthBadge,
                userBadgeResource,
                props.userId,
                userAccount
            )
            if (result.accept) {
                setSuccess(`User permission reinstated in transaction ${result.transactionId}`);
            } else {
                setError(new Error(`Transaction failed ${JSON.stringify(result.rejectReason)}`));
            }
            props.onChange?.(result);
        } catch (e) {
            setError(e);
        } finally {
            setIsBusy(false);
        }
    };

    const handleOnSave = async (e) => {
        e.preventDefault();
        setIsBusy(true);
        setSuccess(null);

        try {
            const result = await provider.setUserExchangeLimit(
                props.issuerId,
                props.adminAuthBadge,
                userBadgeResource,
                props.userId,
                wrappedExchangeLimit
            )
            if (result.accept) {
                setSuccess(`User limit set in transaction ${result.transactionId}`);
            } else {
                setError(new Error(`Transaction failed ${JSON.stringify(result.rejectReason)}`));
            }
            props.onChange?.(result);
        } catch (e) {
            setError(e);
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <Grid container spacing={2} sx={{textAlign: "left"}}>
            <Grid item xs={4} md={4} lg={4}>User badge</Grid>
            <Grid item xs={8} md={8} lg={8}>{props.userBadge}</Grid>
            <Grid item xs={12} md={12} lg={12}><h3>Data</h3></Grid>
            <UserData userData={props.badgeData}/>
            <Grid item xs={12} md={12} lg={12}><h3>Mutable Data</h3></Grid>
            <UserData userData={props.badgeMutableData}/>
            <Grid item xs={12} md={12} lg={12}>
                <form
                    onSubmit={handleOnSave}
                >
                    <Grid container>
                        <Grid item xs={4} md={4} lg={4}>
                            <TextField
                                name="wrapped_exchange_limit"
                                placeholder="Limit for wrapped token exchange"
                                label="Wrapped Exchange Limit"
                                fullWidth
                                required
                                type="number"
                                value={wrappedExchangeLimit}
                                onChange={(e) => setWrappedExchangeLimit(parseInt(e.target.value))}
                            />
                        </Grid>
                        <Grid item xs={4} md={4} lg={4}>
                            <Button variant="contained" type="submit"
                                    disabled={isBusy || props.badgeMutableData?.wrapped_exchange_limit === wrappedExchangeLimit || props.badgeMutableData?.is_blacklisted}
                                    color="secondary">Save</Button>
                        </Grid>
                    </Grid>
                </form>
            </Grid>

            <Grid item xs={12} md={12} lg={12}>
                <BoxHeading2>Permissions</BoxHeading2>
            </Grid>
            <Grid item xs={4} md={4} lg={4}>
                {props.badgeMutableData && (
                    props.badgeMutableData.is_blacklisted ? (
                        <Button variant="contained" color="success" onClick={handleOnReinstate} disabled={isBusy}>
                            Reinstate Access
                        </Button>
                    ) : (
                        <Button variant="contained" color="error" disabled={isBusy} onClick={handleOnRevoke}>Revoke
                            access</Button>

                    ))}
            </Grid>


            {error &&
                <Grid item xs={12} md={12} lg={12}><Alert severity="error">{error.message}</Alert></Grid>}
            {success &&
                <Grid item xs={12} md={12} lg={12}><Alert severity="success">{success}</Alert></Grid>}
        </Grid>
    )
}

function UserData({userData}: { userData: object }) {
    return (
        <>
            {Object.entries(userData).map(([key, value], i) => (
                <React.Fragment key={i}>
                    <Grid item xs={4} md={4} lg={4}>{key}</Grid>
                    <Grid item xs={8} md={8} lg={8}>{displayValue(value)}</Grid>
                </React.Fragment>
            ))}
        </>
    )
}

function displayValue(value: any) {
    if (typeof value === "object") {
        if (value['@@TAGGED@@']) {
            const [tag, v] = value['@@TAGGED@@'];
            return convertTaggedValueToString(tag, v);
        }
        return JSON.stringify(value);
    }
    return value.toString();
}

export default ManageUser;
