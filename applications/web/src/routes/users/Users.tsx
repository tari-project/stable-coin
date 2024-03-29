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
import * as React from "react";
import useTariProvider from "../../store/provider.ts";
import {Alert, CircularProgress} from "@mui/material";
import {useNavigate, useParams} from "react-router-dom";
import {useEffect} from "react";
import * as cbor from '../../cbor';
import AddUser from "./AddUser.tsx";
import GetUser from "./GetUser.tsx";
import {ResourceAddress} from "@tariproject/typescript-bindings";
import Button from "@mui/material/Button";

function Users() {
    const {provider} = useTariProvider();
    const navigate = useNavigate();
    const params = useParams();
    const [adminAuthBadge, setAdminAuthBadge] = React.useState<ResourceAddress | null>(null);
    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);

    if (!provider) {
        useEffect(() => {
            navigate("/");
        }, []);
        return <></>;
    }

    useEffect(() => {
        setIsBusy(true);
        provider.getSubstate(params.issuerId!)
            .then((issuer) => {
                const {value} = issuer;
                const structMap = value.substate.Component.body.state as [object, object][];
                const adminAuthBadge = cbor
                    .getValueByPath(structMap, "$.admin_auth_resource");
                setAdminAuthBadge(adminAuthBadge);
            })
            .catch((e) => setError(e))
            .finally(() => setIsBusy(false));
    }, []);


    return (
        <>
            <Grid item sm={12} md={12} xs={12}>
                <SecondaryHeading>Users</SecondaryHeading>
                <Button onClick={() => navigate(`/issuers/${params.issuerId}`)}>Back</Button>
                {error && <Alert severity="error">{error.message}</Alert>}
            </Grid>
            <Grid item xs={12} md={12} lg={12}>
                {isBusy ? <CircularProgress/> : <AddUser adminAuthBadge={adminAuthBadge!} issuerId={params.issuerId!}/>}
            </Grid>
            <Grid item xs={12} md={12} lg={12}>
                {isBusy ? <CircularProgress/> : <GetUser adminAuthBadge={adminAuthBadge!} issuerId={params.issuerId!}/>}
            </Grid>
        </>
    );
}


export default Users;
