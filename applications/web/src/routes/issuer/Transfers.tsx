// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import * as React from "react";
import {StyledPaper} from "../../components/StyledComponents";
import {Alert, CircularProgress, Grid2 as Grid, TextField} from "@mui/material";
import Button from "@mui/material/Button";
import useTariProvider from "../../store/provider";
import {useNavigate} from "react-router-dom";
import {StableCoinIssuer} from "../../store/stableCoinIssuer";
import Box from "@mui/material/Box";
import {SimpleTransactionResult} from "@tari-project/tarijs-all";

interface Props {
    issuer: StableCoinIssuer;
    onTransactionResult: (result: SimpleTransactionResult) => void;
    onTransactionSubmit?: () => void;
}

function Transfers({issuer, onTransactionResult, onTransactionSubmit}: Props) {
    const [formValues, setFormValues] = React.useState({} as any);
    const {provider} = useTariProvider();
    const navigate = useNavigate();
    const [error, setError] = React.useState<Error | null>(null);
    const [invalid, setInvalid] = React.useState<any>({});
    const [isBusy, setIsBusy] = React.useState(false);

    if (!provider) {
        navigate("/");
        return <></>;
    }

    const set = (key: string) => (evt: React.ChangeEvent<HTMLInputElement>) => {
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
                formValues.transferDestAccount.trim(),
                formValues.transferAmount.trim(),
            );
            onTransactionResult(result);
        } catch (e) {
            console.error(e);
            setError(e as Error);
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <StyledPaper sx={{padding: 6}}>
            {error && (
                <Box sx={{paddingBottom: 4}}>
                    <Alert severity="error">{error.message}</Alert>
                </Box>
            )}
            <Grid container spacing={2}>
                <Grid container spacing={2} sx={{paddingBottom: 2}}>
                    <Grid size={8} sx={{textAlign: "left"}}>
                        <TextField
                            label="To"
                            fullWidth
                            value={formValues.transferDestAccount || ""}
                            placeholder="component_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
                            onChange={set("transferDestAccount")}
                            onBlur={onValidate("transferAmount")}
                        />
                    </Grid>
                    <Grid size={3} sx={{textAlign: "left"}}>
                        <TextField
                            label="Transfer Amount"
                            value={formValues.transferAmount || ""}
                            slotProps={{htmlInput: {pattern: "[0-9]+"}}}
                            onChange={set("transferAmount")}
                            onBlur={onValidate("transferAmount")}
                        />
                    </Grid>
                </Grid>
                <Grid container spacing={2}>
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
                </Grid>
            </Grid>
        </StyledPaper>
    );
}

export default Transfers;
