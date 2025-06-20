// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import Grid from "@mui/material/Grid";
import * as React from "react";
import {StyledPaper} from "../../components/StyledComponents.ts";
import {Alert, CircularProgress, TextField} from "@mui/material";
import Button from "@mui/material/Button";
import useTariProvider from "../../store/provider.ts";
import {useNavigate} from "react-router-dom";
import {StableCoinIssuer} from "../../store/stableCoinIssuer.ts";
import {SimpleTransactionResult} from "../../types.ts";
import Box from "@mui/material/Box";

interface Props {
    issuer: StableCoinIssuer;
    onTransactionResult: (result: SimpleTransactionResult) => void;
    onTransactionSubmit?: () => void;
}

function SupplyControl({issuer, onTransactionResult, onTransactionSubmit}: Props) {
    const [formValues, setFormValues] = React.useState({} as any);
    const {provider} = useTariProvider();
    const navigate = useNavigate();
    const [error, setError] = React.useState<Error | null>(null);
    const [invalid, setInvalid] = React.useState<any>({});
    const [busy, setBusy] = React.useState<{ [key: string]: boolean }>({});

    if (!provider) {
        navigate("/");
        return <></>;
    }

    const set = (key: string) => (evt: React.ChangeEvent<HTMLInputElement>) => {
        setFormValues({...formValues, [key]: evt.target.value});
    };
    const onValidate = (key: string) => (evt: React.FocusEvent<HTMLInputElement>) => {
        if (evt.target.validity.valid) {
            delete invalid[key];
            setInvalid({...invalid});
        } else {
            setInvalid({[key]: `Invalid`});
            return;
        }
    };
    const submitIncreaseSupply = async () => {
        try {
            onTransactionSubmit && onTransactionSubmit();
            setBusy({increaseSupply: true});
            const result: SimpleTransactionResult = await provider.increaseSupply(
                issuer.id,
                issuer.adminAuthResource,
                formValues.increaseSupply.trim(),
            );
            onTransactionResult(result);
        } catch (e) {
            console.error(e);
            setError(e as Error);
        } finally {
            setBusy({});
        }
    };
    const submitDecreaseSupply = async () => {
        try {
            onTransactionSubmit && onTransactionSubmit();
            setBusy({decreaseSupply: true});
            const result = await provider.decreaseSupply(
                issuer.id,
                issuer.adminAuthResource,
                formValues.decreaseSupply.trim(),
            );
            onTransactionResult(result);
        } catch (e) {
            console.error(e);
            setError(e as Error);
        } finally {
            setBusy({});
        }
    };

    const isBusy = Object.keys(busy).length > 0;

    return (
        <StyledPaper sx={{padding: 6}}>
            {error && (
                <Box sx={{paddingBottom: 4}}>
                    <Alert severity="error">{error.message}</Alert>
                </Box>
            )}
            <Grid container spacing={2}>
                <Grid container spacing={2} sx={{paddingBottom: 2}}>
                    <Grid item xs={3} md={3} lg={3} sx={{textAlign: "left"}}>
                        <TextField
                            label="Increase supply"
                            value={formValues.increaseSupply}
                            inputProps={{pattern: "[0-9]+"}}
                            defaultValue=""
                            onChange={set("increaseSupply")}
                            onBlur={onValidate("increaseSupply")}
                        />
                    </Grid>
                    <Grid item xs={3} md={3} lg={3} sx={{textAlign: "left"}}>
                        <Button
                            variant="contained"
                            color="primary"
                            disabled={!formValues.increaseSupply || Boolean(invalid.increaseSupply) || isBusy}
                            onClick={() => submitIncreaseSupply()}
                        >
                            {busy?.increaseSupply ? <CircularProgress size={24}/> : <span>Increase Supply</span>}
                        </Button>
                    </Grid>
                    {invalid.increaseSupply && (
                        <Grid item xs={3} md={3} lg={3} sx={{textAlign: "left"}}>
                            <Alert severity="error">{invalid.increaseSupply}</Alert>
                        </Grid>
                    )}
                </Grid>
                <Grid container spacing={2} sx={{paddingBottom: 2}}>
                    <Grid item xs={3} md={3} lg={3} sx={{textAlign: "left"}}>
                        <TextField
                            label="Decrease supply"
                            value={formValues.decreaseSupply}
                            defaultValue=""
                            inputProps={{pattern: "[0-9]+"}}
                            onChange={set("decreaseSupply")}
                            onBlur={onValidate("decreaseSupply")}
                        />
                    </Grid>
                    <Grid item xs={3} md={3} lg={3} sx={{textAlign: "left"}}>
                        <Button
                            variant="contained"
                            color="primary"
                            disabled={!formValues.decreaseSupply || Boolean(invalid.decreaseSupply) || isBusy}
                            onClick={() => submitDecreaseSupply()}
                        >
                            {busy?.decreaseSupply ? <CircularProgress size={24}/> : <span>Decrease Supply</span>}
                        </Button>
                    </Grid>
                    {invalid.decreaseSupply && (
                        <Grid item xs={3} md={3} lg={3} sx={{textAlign: "left"}}>
                            <Alert severity="error">{invalid.decreaseSupply}</Alert>
                        </Grid>
                    )}
                </Grid>
            </Grid>
        </StyledPaper>
    );
}

export default SupplyControl;
