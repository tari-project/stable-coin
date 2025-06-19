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
import {ComponentAddress, ResourceAddress} from "@tari-project/typescript-bindings";

interface Props {
    issuerId: ComponentAddress;
    adminAuthBadge?: ResourceAddress;
}

function AddUser(props: Props) {
    const {provider} = useTariProvider();

    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);
    const [formValues, setFormValues] = React.useState<Partial<any>>({});
    const [validity, setValidity] = React.useState<Partial<any>>({});
    const [success, setSuccess] = React.useState<string | null>(null);

    const set = (e: React.ChangeEvent<HTMLInputElement>) => {
        setFormValues({...formValues, [e.target.name]: e.target.value});
    };

    const validate = (e: React.FocusEvent<HTMLInputElement>) => {
        if (e.target.value) {
            setValidity({...validity, [e.target.name]: e.target.validity.valid});
        } else {
            delete validity[e.target.name];
            setValidity(validity);
        }
    };

    const isValid = Object.values(validity).every((v) => v);

    const createUser = async (values: any) => {
        if (!props.adminAuthBadge) {
            return;
        }

        setIsBusy(true);
        setError(null);
        setSuccess(null);
        try {
            const result = await provider!.createUser(
                `component_${props.issuerId}`, props.adminAuthBadge, values.userId, values.userAccount);
            if (result.rejectReason) {
                setError(new Error(`Transaction failed ${JSON.stringify(result.rejectReason)}`));
                return;
            }
            console.log(result.accept?.up_substates);
            const [_t, id, _v, _val] = result.accept?.up_substates
                ?.filter(([type, _id, _v, _val]) => type === "NonFungible")
                .find(([_type, id, _v, _val]) => id.endsWith(`_u64_${values.userId}`))!;
            setSuccess(`User created in transaction ${result.transactionId}. User badge: ${JSON.stringify(id)}`);
            setFormValues({});
        } catch (e) {
            setError(e as Error);
        } finally {
            setIsBusy(false);
        }
    };

    return (
        <>
            <StyledPaper sx={{padding: 6}}>
                {error && <Alert severity="error">{error.message}</Alert>}
                {success && <Alert severity="success">{success}</Alert>}
                <Grid container spacing={2}>
                    <Grid item xs={12} md={12} lg={12}>
                        <h2>Add User</h2>
                    </Grid>
                    <Grid item xs={12} md={12} lg={12}>
                        <form
                            onSubmit={async (e) => {
                                e.preventDefault();
                                await createUser(formValues);
                            }}
                        >
                            <Grid item xs={12} md={12} lg={12} paddingBottom={4}>
                                <Alert severity="info">Admin Badge: {props.adminAuthBadge}</Alert>
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
                            <Grid item xs={12} md={12} lg={12} paddingBottom={4}>
                                <TextField
                                    name="userAccount"
                                    placeholder="component_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
                                    label="User Account Component"
                                    fullWidth
                                    slotProps={{input: {pattern: "^component_[0-9a-fA-F]{64}$"}}}
                                    required
                                    onChange={set}
                                    onBlur={validate}
                                    value={formValues.userAccount || ""}
                                />
                            </Grid>
                            {validity.userAccount === false && (
                                <Grid item xs={12} md={12} lg={12}>
                                    <Alert severity="error">User account must be a component ID</Alert>
                                </Grid>
                            )}
                            <Grid item xs={12} md={12} lg={12} sx={{textAlign: "left"}}>
                                <Button type="submit" disabled={isBusy || !isValid} variant="contained">
                                    Create
                                </Button>
                            </Grid>
                        </form>
                    </Grid>
                </Grid>
            </StyledPaper>
        </>
    );
}

export default AddUser;
