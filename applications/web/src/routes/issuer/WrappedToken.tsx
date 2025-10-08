// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import {Alert, Button, CircularProgress, Grid2 as Grid, TextField} from "@mui/material";
import * as React from "react";
import {StyledPaper} from "../../components/StyledComponents";
import useTariProvider from "../../store/provider";
import {useNavigate} from "react-router-dom";
import {StableCoinIssuer} from "../../store/stableCoinIssuer";
import Box from "@mui/material/Box";
import IconButton from "@mui/material/IconButton";
import {RefreshOutlined} from "@mui/icons-material";
import {AccountDetails} from "../../components/AccountDetails";
import useActiveAccount from "../../store/account";
import {SimpleTransactionResult} from "@tari-project/tarijs-all";

interface Props {
    issuer: StableCoinIssuer;
}

function WrappedToken({issuer}: Props) {
    const [formValues, setFormValues] = React.useState({} as any);
    const {provider} = useTariProvider();
    const navigate = useNavigate();
    const [error, setError] = React.useState<Error | null>(null);
    const [invalid, setInvalid] = React.useState({} as any);
    const [isBusy, setIsBusy] = React.useState(false);
    const {account, setActiveAccount} = useActiveAccount();
    const [exchangeResult, setExchangeResult] = React.useState<SimpleTransactionResult | null>(null);

    if (!provider) {
        navigate("/");
        return <></>;
    }


    const set = (key: string) => (evt: React.ChangeEvent<HTMLInputElement>) => {
        if (!evt.target.validity.valid) {
            return;
        }
        setFormValues({...formValues, [key]: evt.target.value});
    };
    const onValidate = (key: string) => (evt: React.FocusEvent<HTMLInputElement>) => {
        console.log(evt.target.validity);
        if (evt.target.validity.valid) {
            delete invalid[key];
            setInvalid({...invalid});
        } else {
            setInvalid({[key]: `Invalid`});
            return;
        }
    };

    const submitExchangeTokenForWrapped = async () => {
        setError(null);
        try {
            setIsBusy(true);
            const result = await provider.exchangeStableForWrappedToken(
                issuer.id,
                account!.component_address,
                issuer.vault.resourceAddress,
                issuer.userAuthResource,
                Number(formValues.userId.trim()),
                Number(formValues.exchangeAmount.trim()),
            );
            if (result.accept.isSome()) {
                setExchangeResult(result);
            } else {
                const reason = result.anyRejectReason.unwrap();
                setError(new Error(JSON.stringify(reason)));
            }

            loadAccount();
        } catch (e) {
            console.error(e);
            setError(e as Error);
        } finally {
            setIsBusy(false);
        }
    };

    const submitExchangeWrappedForToken = async () => {
        setError(null);
        try {
            setIsBusy(true);
            const result = await provider.exchangeWrappedForStable(
                issuer.id,
                account!.component_address,
                issuer.wrappedToken!.resource,
                issuer.userAuthResource,
                Number(formValues.userId.trim()),
                Number(formValues.exchangeAmount.trim()),
            );
            if (result.accept) {
                setExchangeResult(result);
            } else {
                const reason = result.anyRejectReason.unwrap();
                setError(new Error(JSON.stringify(reason)));
            }

            loadAccount();
        } catch (e) {
            console.error(e);
            setError(e as Error);
        } finally {
            setIsBusy(false);
        }
    };

    const loadAccount = () => {
        setIsBusy(true);
        provider
            .getAccount()
            .then((account) => {
                setActiveAccount(account);
            })
            .catch((e) => setError(e))
            .finally(() => setIsBusy(false));
    };

    return (
        <StyledPaper sx={{padding: 6}}>
            {error && (
                <Box sx={{paddingBottom: 4}}>
                    <Alert severity="error">{error.message}</Alert>
                </Box>
            )}
            {exchangeResult?.accept && (
                <Box sx={{paddingBottom: 4}}>
                    <Alert severity="success">
                        {`Transaction ${exchangeResult.transactionId} was accepted`}
                    </Alert>
                </Box>
            )}
            <Box sx={{paddingBottom: 4}}>
                NOTE: this functionality demonstrates a wrapped token exchange that a user would use when connecting
                their
                wallet (not the issuer's wallet). Switch the default account to one that has tokens to exchange.
            </Box>
            <Grid container spacing={2}>
                <Grid container spacing={2} sx={{paddingBottom: 2}}>
                    <Grid size={12} sx={{textAlign: "left"}}>
                        <IconButton onClick={loadAccount}>
                            <RefreshOutlined/>
                        </IconButton>
                        {account ? <AccountDetails account={account}/> : <CircularProgress size={24}/>}
                    </Grid>
                    <Grid size={12} sx={{textAlign: "left"}}>
                        <TextField
                            label="User Id"
                            value={formValues.userId || ""}
                            required
                            inputProps={{pattern: "[0-9]+"}}
                            onChange={set("userId")}
                            onBlur={onValidate("userId")}
                        />
                    </Grid>
                    <Grid size={12} sx={{textAlign: "left"}}>
                        <TextField
                            label="Exchange Amount"
                            value={formValues.exchangeAmount || ""}
                            required
                            inputProps={{pattern: "[0-9]+"}}
                            onChange={set("exchangeAmount")}
                            onBlur={onValidate("exchangeAmount")}
                        />
                    </Grid>
                </Grid>
                <Grid container spacing={2}>
                    <Grid size={6} sx={{textAlign: "left"}}>
                        <Button
                            variant="contained"
                            color="primary"
                            disabled={!formValues.exchangeAmount || Boolean(invalid.exchangeAmount) || isBusy}
                            onClick={() => submitExchangeTokenForWrapped()}
                        >
                            {isBusy ? <CircularProgress size={24}/> : <span>Exchange Stable -&gt; Wrapped</span>}
                        </Button>
                    </Grid>
                    <Grid size={6} sx={{textAlign: "left"}}>
                        <Button
                            variant="contained"
                            color="primary"
                            disabled={!formValues.exchangeAmount || Boolean(invalid.exchangeAmount) || isBusy}
                            onClick={() => submitExchangeWrappedForToken()}
                        >
                            {isBusy ? <CircularProgress size={24}/> : <span>Exchange Wrapped -&gt; Stable</span>}
                        </Button>
                    </Grid>
                    {invalid.exchangeAmount && (
                        <Grid size={12} sx={{textAlign: "left"}}>
                            <Alert severity="error">{invalid.exchangeAmount}</Alert>
                        </Grid>
                    )}
                </Grid>
            </Grid>
        </StyledPaper>
    );
}


export default WrappedToken;
