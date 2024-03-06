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
import {StyledPaper} from "../../components/StyledComponents.ts";
import * as React from "react";
import useTariProvider from "../../store/provider.ts";
import {Alert, TextField} from "@mui/material";
import Button from "@mui/material/Button";
import {useEffect} from "react";
import * as cbor from '../../cbor';
import ManageUser from "./ManageUser.tsx";
import {ComponentAddress, ResourceAddress} from "@tariproject/typescript-bindings";

interface Props {
    issuerId: ComponentAddress,
    adminAuthBadge?: ResourceAddress,

}

function GetUser(props: Props) {
    const {provider} = useTariProvider();

    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);
    const [formValues, setFormValues] = React.useState({});
    const [validity, setValidity] = React.useState({});
    const [userAuthBadge, setUserAuthBadge] = React.useState<string | null>(null);
    const [userData, setUserData] = React.useState<object | null>(null);

    useEffect(() => {
        provider.getSubstate(props.issuerId)
            .then((issuer) => {
                const {value} = issuer;
                const structMap = value.substate.Component.body.state as [object, object][];
                const userAuthBadge = cbor
                    .getValueByPath(structMap, "$.user_auth_resource");
                setUserAuthBadge(userAuthBadge);
            })
            .catch((e) => setError(e))
            .finally(() => setIsBusy(false));
    }, [formValues]);

    const set = (e) => {
        setFormValues({...formValues, [e.target.name]: e.target.value});
    }

    const validate = (e) => {
        setValidity({...validity, [e.target.name]: e.target.validity.valid});
    }

    const isValid = Object.values(validity).every((v) => v);

    const getUser = async (userId) => {
        if (!userAuthBadge) {
            return;
        }

        setIsBusy(true);
        setError(null);
        setUserData(null);
        try {
            const substate = await provider.getSubstate(`${userAuthBadge} nft_u64:${userId}`);
            setUserData(substate as object);
        } catch (e) {
            console.error(e);
            setError(e);
        } finally {
            setIsBusy(false);
        }
    }

    return (
        <>
            <StyledPaper sx={{padding: 6}}>
                <Grid container spacing={2}>
                    <Grid item xs={12} md={12} lg={12}>
                        <h2>User</h2>
                    </Grid>
                    <Grid item xs={12} md={12} lg={12}>
                        <form onSubmit={async (e) => {
                            e.preventDefault();
                            await getUser(formValues.userId);
                        }}>
                            <Grid item xs={12} md={12} lg={12} paddingBottom={4}>
                                <TextField
                                    name="userId"
                                    placeholder="12345"
                                    label="User ID (must be unique per user)"
                                    fullWidth
                                    required
                                    inputProps={{pattern: "^[0-9]*$"}}
                                    onChange={set}
                                    onBlur={validate}
                                    value={formValues.userId || ""}
                                />
                                {validity.userId === false && <Alert severity="error">User ID must be a number</Alert>}
                            </Grid>
                            <Grid item xs={12} md={12} lg={12} sx={{textAlign: "left", paddingBottom: 4}}>
                                <Button type="submit" disabled={isBusy || !isValid} variant="contained">Load</Button>
                            </Grid>
                            {error && <Alert severity="error">{error.message}</Alert>}
                        </form>
                    </Grid>
                </Grid>
                {userData && (
                    <ManageUser
                        issuerId={props.issuerId}
                        adminBadge={props.adminAuthBadge}
                        userBadge={userData.record.substate_id.NonFungible}
                        badgeData={userData.value.substate.NonFungible.data}
                        badgeMutableData={userData.value.substate.NonFungible.mutable_data}
                    />
                )}
            </StyledPaper>
        </>
    )
}

function UserData({userData}: { userData: object }) {
    return (
        <>
            {Object.entries(userData).map(([key, value], i) => (
                <>
                    <Grid key={i} item xs={4} md={4} lg={4}>{key}</Grid>
                    <Grid key={`k-${i}`} item xs={8} md={8} lg={8}>{value.toString()}</Grid>
                </>
            ))}
        </>
    )
}

export default GetUser;
