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

interface Props {
    issuerId: ComponentAddress
    userId: number,
    userBadge: ResourceAddress,
    adminAuthBadge: ResourceAddress,
    badgeData: object,
    badgeMutableData: object,
}

function ManageUser(props: Props) {
    const {provider} = useTariProvider();

    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);

    const handleOnRevoke = async () => {

        if (!props.badgeData.user_account) {
            throw new Error("No account address found in badge data");
        }

        setIsBusy(true);

        try {
            const [tag, v] = props.badgeData.user_account;
            const substate = await provider.getSubstate(convertTaggedValueToString(tag, v));
            console.log(substate);
            // const resp = await provider.revokeUserAccess(props.issuerId, props.adminAuthBadge, props.userId, vaultId)
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

            <Grid item xs={4} md={4} lg={4}><Button variant="contained" color="error" disabled={isBusy}
                                                    onClick={handleOnRevoke}>Revoke
                access</Button> </Grid>
            <Grid item xs={4} md={4} lg={4}><Button variant="contained" color="error" onClick={handleOnRevoke}>Revoke
                access</Button> </Grid>
            <Grid item xs={4} md={4} lg={4}><Button variant="contained" color="error" onClick={handleOnRevoke}>Revoke
                access</Button> </Grid>

            {error &&
                <Grid item xs={12} md={12} lg={12}><Alert severity="error">{error.message}</Alert></Grid>}
        </Grid>
    )
}

function UserData({userData}: { userData: object }) {
    return (
        <>
            {Object.entries(userData).map(([key, value], i) => (
                <>
                    <Grid key={i} item xs={4} md={4} lg={4}>{key}</Grid>
                    <Grid key={`k-${i}`} item xs={8} md={8} lg={8}>{displayValue(value)}</Grid>
                </>
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
