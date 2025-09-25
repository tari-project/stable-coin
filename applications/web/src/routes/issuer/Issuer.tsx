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
import SecondaryHeading from "../../components/SecondaryHeading.js";
import {StyledPaper} from "../../components/StyledComponents.js";
import * as React from "react";
import {useEffect} from "react";
import useTariProvider from "../../store/provider.js";
import useActiveIssuer, {StableCoinIssuer} from "../../store/stableCoinIssuer.js";
import {Alert, Button, CircularProgress, Grid2 as Grid} from "@mui/material";
import {useNavigate, useParams} from "react-router-dom";
import SupplyControl from "./SupplyControl.js";
import Transfers from "./Transfers.js";
import WrappedToken from "./WrappedToken.js";
import useIssuers from "../../store/issuers.js";
import useActiveAccount from "../../store/account.js";
import TransactionList from "./TransactionList.js";
import {convertToIssuer} from "../home/Setup.js";
import {decodeOotleAddress} from "@tari-project/typescript-bindings";
import {SimpleTransactionResult} from "@tari-project/tarijs-all";

interface IssuerDetailsProps {
    issuer: StableCoinIssuer;
}

function Detail({label, value}: { label: string; value: string }) {
    return (
        <Grid container size={12}>
            <Grid size={3} sx={{textAlign: "left"}}>
                {label}
            </Grid>
            <Grid size={9} sx={{textAlign: "left"}}>
                {value}
            </Grid>
        </Grid>
    );
}

function IssuerDetails({issuer}: IssuerDetailsProps) {
    const navigate = useNavigate();
    const params = useParams();
    return (
        <StyledPaper sx={{padding: 6}}>
            <Grid container spacing={2} sx={{paddingBottom: 4}}>
                <Detail label="Component ID" value={issuer.id}/>
                <Detail label="Resource Address" value={issuer.vault.resourceAddress}/>
                <Detail label="Revealed Vault Supply" value={issuer.vault.revealedAmount.toString()}/>
                <Detail label="Admin Badge Resource" value={issuer.adminAuthResource}/>
                <Detail label="User Badge Resource" value={issuer.userAuthResource}/>
                <Detail label="Wrapped Token" value={issuer.wrappedToken?.resource || "Disabled"}/>
                <Detail label="Wrapped Balance" value={issuer.wrappedToken?.balance.toString() || "N/A"}/>
            </Grid>
            <Grid container spacing={2} sx={{textAlign: "left"}}>
                <Button variant="contained" color="secondary"
                        onClick={() => navigate(`/issuers/${params.issuerId}/users`)}>
                    Manage Users
                </Button>
            </Grid>
        </StyledPaper>
    );
}


function Issuer() {
    const {provider} = useTariProvider();
    const {activeIssuer, setActiveIssuer} = useActiveIssuer();
    const [error, setError] = React.useState<Error | null>(null);
    const [success, setSuccess] = React.useState<string | null>(null);
    const params = useParams();
    const navigate = useNavigate();
    const {setIssuers, getIssuers} = useIssuers();
    const [isBusy, setIsBusy] = React.useState(false);
    const {account} = useActiveAccount();

    if (!provider) {
        useEffect(() => {
            navigate("/");
        }, []);
        return <></>;
    }


    function load() {
        const decoded = decodeOotleAddress(account!.wallet_address);
        const issuers = getIssuers(decoded.accountPublicKey);
        if (!issuers) {
            return;
        }
        Promise.all(issuers.map((s) => provider!.getSubstate(s.id).then((substate) => ({
            address: substate.address,
            value: substate.value.Component,
        }))))
            .then((substates) => Promise.all(substates.map((s) => convertToIssuer(provider!, s))))
            .then((issuers) => {
                if (activeIssuer?.id !== params.issuerId && !error) {
                    setIssuers(decoded.accountPublicKey, issuers);
                    const issuer = (issuers || [])
                        .find((i: StableCoinIssuer) => i.id == params.issuerId!);
                    if (!issuer) {
                        navigate("/");
                        return;
                    }
                    setActiveIssuer(issuer);
                }
            })
            .catch((e: Error) => {
                console.error(e);
                setError(e)
            })
            .finally(() => setIsBusy(false))
        ;
    }

    React.useEffect(load, [activeIssuer, params.issuerId]);

    const handleTransactionResult = (result: SimpleTransactionResult) => {
        if (result.accept) {
            setSuccess(`Transaction ${result.transactionId} succeeded`);
            setActiveIssuer(null);
            load();
        }

        const reject = result.anyRejectReason;
        if (reject.isSome()) {
            setSuccess(null);
            setError(new Error(`Transaction rejected: ${JSON.stringify(reject.unwrap())}`));
        }
    };

    return (
        <>
            <Grid size={12}>
                <SecondaryHeading>Issuer</SecondaryHeading>
            </Grid>
            {error && (
                <Grid size={12}>
                    <Alert severity="error">{error.message}</Alert>
                </Grid>
            )}
            {success && (
                <Grid size={12}>
                    <Alert severity="success">{success}</Alert>
                </Grid>
            )}
            {!activeIssuer || isBusy ? (
                <CircularProgress/>
            ) : (
                <>
                    <Grid size={12}>
                        <h2>Details</h2>
                        <IssuerDetails issuer={activeIssuer}/>
                    </Grid>
                    <Grid size={12}>
                        <h2>Supply</h2>
                        <SupplyControl
                            issuer={activeIssuer}
                            onTransactionSubmit={() => setSuccess(null)}
                            onTransactionResult={handleTransactionResult}
                        />
                    </Grid>
                    <Grid size={12}>
                        <h2>Transfers</h2>
                        <Transfers issuer={activeIssuer} onTransactionResult={handleTransactionResult}/>
                    </Grid>
                    <Grid size={12}>
                        <h2>Transactions</h2>
                        <TransactionList issuer={activeIssuer}/>
                    </Grid>
                    {activeIssuer.wrappedToken && (
                        <Grid size={12}>
                            <h2>Wrapped Token</h2>
                            <WrappedToken issuer={activeIssuer}/>
                        </Grid>
                    )}
                </>
            )}
        </>
    );
}

export default Issuer;
