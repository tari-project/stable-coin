// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import * as React from "react";
import {Alert, Button, CircularProgress, Grid2 as Grid, TextField} from "@mui/material";
import useTariProvider from "../../store/provider";
import {useNavigate} from "react-router-dom";
import {StableCoinIssuer} from "../../store/stableCoinIssuer";
import {ComponentAddress} from "@tari-project/typescript-bindings";
import {SimpleTransactionResult} from "@tari-project/tarijs-all";

interface Props {
    issuer: StableCoinIssuer;
    userAccount: ComponentAddress;
    onTransactionResult: (result: SimpleTransactionResult) => void;
    onTransactionSubmit?: () => void;
}

function Transfers({issuer, onTransactionResult, onTransactionSubmit, userAccount}: Props) {
    const [formValues, setFormValues] = React.useState({} as any);
    const {provider} = useTariProvider();
    const navigate = useNavigate();
    const [error, setError] = React.useState<Error | null>(null);
    const [invalid, setInvalid] = React.useState<Partial<any>>({});
    const [isBusy, setIsBusy] = React.useState(false);

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

    const submitTransfer = async () => {
        try {
            onTransactionSubmit && onTransactionSubmit();
            setIsBusy(true);
            const result = await provider.transfer(
                issuer.id,
                issuer.adminAuthResource,
                userAccount,
                formValues.transferAmount.trim(),
            );
            if (result.accept) {
                setFormValues({});
            }
            onTransactionResult(result);
        } catch (e) {
            console.error(e);
            setError(e as Error);
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <>
            <Grid container spacing={2} sx={{textAlign: "left"}}>
                <Grid size={12}>
                    <h2>Transfer</h2>
                </Grid>
                <Grid size={3} sx={{textAlign: "left"}}>
                    <TextField
                        label="Transfer Amount"
                        value={formValues.transferAmount}
                        defaultValue=""
                        slotProps={{htmlInput: {pattern: "[0-9]+"}}}
                        onChange={set("transferAmount")}
                        onBlur={onValidate("transferAmount")}
                    />
                </Grid>
                <Grid size={3} sx={{textAlign: "left"}}>
                    <Button
                        variant="contained"
                        color="primary"
                        disabled={!formValues.transferAmount || Boolean(invalid.transferAmount) || isBusy}
                        onClick={() => submitTransfer()}
                    >
                        {isBusy ? <CircularProgress size={24}/> : <span>Transfer</span>}
                    </Button>
                </Grid>
                {invalid.transferAmount && (
                    <Grid size={3} sx={{textAlign: "left"}}>
                        <Alert severity="error">{invalid.transferAmount}</Alert>
                    </Grid>
                )}

                {error && (
                    <Grid size={3} sx={{textAlign: "left"}}>
                        <Alert severity="error">{error.message}</Alert>
                    </Grid>
                )}
            </Grid>
        </>
    );
}

export default Transfers;
