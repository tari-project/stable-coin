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
import SecondaryHeading from "../../components/SecondaryHeading.tsx";
import {StyledPaper} from "../../components/StyledComponents.ts";
import * as React from "react";
import useTariProvider from "../../store/provider.ts";
import useActiveIssuer, {ActiveIssuer} from "../../store/activeIssuer.ts";
import {Alert, CircularProgress, MenuItem, Select, TextField} from "@mui/material";
import Button from "@mui/material/Button";
import {useNavigate, useParams} from "react-router-dom";
import {useEffect} from "react";
import * as cbor from '../../cbor';
import {SimpleTransactionResult} from "../../types.ts";
import useSettings from "../../store/settings.ts";
import activeIssuer from "../../store/activeIssuer.ts";

interface Props {
}

function GetUser(_props: Props) {
    const {provider} = useTariProvider();
    const {settings} = useSettings();

    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);
    const [formValues, setFormValues] = React.useState({});
    const [validity, setValidity] = React.useState({});
    const [issuerComponents, setIssuerComponents] = React.useState<ActiveIssuer[] | null>(null);
    const [userAuthBadge, setUserAuthBadge] = React.useState<string | null>(null);
    const [userData, setUserData] = React.useState<object | null>(null);

    useEffect(() => {
        if (!isBusy && settings.template && !error && issuerComponents === null) {
            setIsBusy(true);
            const getIssuerComponents = provider.listSubstates(settings.template, "Component")
                .then((substates) => {
                    setIssuerComponents(substates.filter((s) => s.template_address === settings.template));
                });

            Promise.allSettled([getIssuerComponents])
                .catch((e) => setError(e))
                .finally(() => setIsBusy(false));
        }
    }, [isBusy, error, issuerComponents]);

    useEffect(() => {
        if (!formValues.issuerComponent) {
            return;
        }
        provider.getSubstate(formValues.issuerComponent)
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

    const getUser = async (values) => {
        if (!userAuthBadge) {
            return;
        }

        setIsBusy(true);
        setError(null);
        setUserData(null);
        try {
            const substate = await provider.getSubstate(`${userAuthBadge} nft_u64:${values.userId}`);
            setUserData(substate as object);
        } catch (e) {
            setError(e);
        } finally {
            setIsBusy(false);
        }
    }

    return (
        <>
            <StyledPaper sx={{padding: 6}}>
                {error && <Alert severity="error">{error.message || error}</Alert>}
                <Grid container spacing={2}>
                    <Grid item xs={12} md={12} lg={12}>
                        <h2>User</h2>
                    </Grid>
                    <Grid item xs={12} md={12} lg={12}>
                        <form onSubmit={async (e) => {
                            e.preventDefault();
                            await getUser(formValues);
                        }}>
                            <Grid item xs={12} md={12} lg={12} paddingBottom={4}>
                                <Select
                                    name="issuerComponent"
                                    label="Isser Component"
                                    fullWidth
                                    required
                                    onChange={set}
                                    displayEmpty
                                    value={formValues.issuerComponent || ""}
                                >
                                    {!issuerComponents && <MenuItem disabled>Loading...</MenuItem>}
                                    {issuerComponents?.map((c, i) => (
                                        <MenuItem key={i}
                                                  value={c.substate_id.Component}>{c.substate_id.Component}</MenuItem>
                                    ))}
                                </Select>
                                {userAuthBadge && <Alert severity="info">Admin Badge: {userAuthBadge}</Alert>}
                            </Grid>
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
                            <Grid item xs={12} md={12} lg={12} sx={{textAlign: "left"}}>
                                <Button type="submit" disabled={isBusy || !isValid} variant="contained">Get</Button>
                            </Grid>
                        </form>
                    </Grid>
                </Grid>
                <Grid container spacing={2}>
                    <Grid item xs={12} md={12} lg={12}>
                        {userData && <pre>{JSON.stringify(cbor.userData, null, 2)}</pre>}
                    </Grid>
                </Grid>
            </StyledPaper>
        </>
    )
}


export default GetUser;
