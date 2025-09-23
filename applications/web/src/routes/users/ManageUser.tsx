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
import * as React from "react";
import useTariProvider from "../../store/provider";
import {Alert, Grid2 as Grid, Table, TableBody, TableHead, TableRow, TextField} from "@mui/material";
import Button from "@mui/material/Button";
import {ComponentAddress, ResourceAddress} from "@tari-project/typescript-bindings";
import {DataTableCell} from "../../components/StyledComponents";
import {useNavigate} from "react-router-dom";
import {getCborValueByPath, SimpleTransactionResult} from "@tari-project/tarijs-all";

interface Props {
    issuerId: ComponentAddress;
    userId: number;
    userBadge: ResourceAddress;
    adminAuthBadge: ResourceAddress;
    badgeData: any;
    badgeMutableData: any;
    onChange?: (result: SimpleTransactionResult) => void;
}

function ManageUser(props: Props) {
    const {provider} = useTariProvider();

    const [isBusy, setIsBusy] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);
    const [success, setSuccess] = React.useState<string | null>(null);
    const [wrappedExchangeLimit, setWrappedExchangeLimit] = React.useState<number>(
        props.badgeMutableData.wrapped_exchange_limit,
    );

    const navigate = useNavigate();

    const userAccount = props.badgeData.user_account;
    const userBadgeResource = props.userBadge.split("_")[1];

    if (!provider) {
        React.useEffect(() => {
            navigate("/");
        }, []);
        return <></>;
    }

    const runQuery = async (query: () => Promise<string | null>) => {
        setIsBusy(true);
        setSuccess(null);
        setError(null);

        try {
            const success = await query();
            setSuccess(success);
        } catch (e) {
            setError(e as Error);
        } finally {
            setIsBusy(false);
        }
    };

    const handleOnRevoke = async () => {
        await runQuery(async () => {
            const substate = await provider.getSubstate(userAccount) as any;
            const vaults = getCborValueByPath(substate.value.Component.body.state, "$.vaults") as Record<string, string>;
            const vaultToRevoke = vaults[userBadgeResource];
            if (!vaultToRevoke) {
                throw new Error(`User does not have a stable coin user badge ${userBadgeResource}`);
            }

            const result = await provider.revokeUserAccess(
                props.issuerId,
                props.adminAuthBadge,
                userBadgeResource,
                props.userId,
                vaultToRevoke,
            );
            if (result.accept) {
                props.onChange?.(result);
                return `User permission revoked in transaction ${result.transactionId}`;
            }

            throw new Error(`Transaction failed ${JSON.stringify(result.anyRejectReason.unwrap())}`);
        });
    };

    const handleOnReinstate = async () => {
        await runQuery(async () => {
            const result = await provider.reinstateUserAccess(
                props.issuerId,
                props.adminAuthBadge,
                userBadgeResource,
                props.userId,
                userAccount,
            );
            if (result.accept) {
                props.onChange?.(result);
                return `User permission reinstated in transaction ${result.transactionId}`;
            }

            throw new Error(`Transaction failed ${JSON.stringify(result.anyRejectReason.unwrap())}`);
        });
    };

    const handleOnSave = async (e: React.FormEvent<HTMLFormElement>) => {
        e.preventDefault();
        await runQuery(async () => {
            const result = await provider.setUserExchangeLimit(
                props.issuerId,
                props.adminAuthBadge,
                userBadgeResource,
                props.userId,
                wrappedExchangeLimit,
            );
            if (result.accept) {
                props.onChange?.(result);
                return `User limit set in transaction ${result.transactionId}`;
            }

            throw new Error(`Transaction failed ${JSON.stringify(result.anyRejectReason.unwrap())}`);
        });
    };

    return (
        <Grid container spacing={2} sx={{textAlign: "left"}}>
            <Grid size={12}>
                <h3>Data</h3>
            </Grid>
            <UserData userData={props.badgeData}/>
            <Grid size={12}>
                <h3>Mutable Data</h3>
            </Grid>
            <UserData userData={props.badgeMutableData}/>
            <h3>Wrapped Token</h3>
            <Grid size={12}>
                <form onSubmit={handleOnSave}>
                    <Grid container>
                        <Grid size={4}>
                            <TextField
                                name="wrapped_exchange_limit"
                                placeholder="Limit for wrapped token exchange"
                                label="Wrapped Exchange Limit"
                                fullWidth
                                required
                                type="number"
                                value={wrappedExchangeLimit}
                                onChange={(e) => setWrappedExchangeLimit(parseInt(e.target.value))}
                            />
                        </Grid>
                        <Grid size={4}>
                            <Button
                                variant="contained"
                                type="submit"
                                disabled={
                                    isBusy ||
                                    props.badgeMutableData?.wrapped_exchange_limit === wrappedExchangeLimit ||
                                    props.badgeMutableData?.is_blacklisted
                                }
                                color="secondary"
                            >
                                Save
                            </Button>
                        </Grid>
                    </Grid>
                </form>
            </Grid>

            <Grid size={12}>
                <h3>Permissions</h3>
            </Grid>
            <Grid size={4}>
                {props.badgeMutableData &&
                    (props.badgeMutableData.is_blacklisted ? (
                        <Button variant="contained" color="success" onClick={handleOnReinstate} disabled={isBusy}>
                            Reinstate Access
                        </Button>
                    ) : (
                        <Button variant="contained" color="error" disabled={isBusy} onClick={handleOnRevoke}>
                            Revoke access
                        </Button>
                    ))}
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
        </Grid>
    );
}

function UserData({userData}: { userData: object }) {
    return (
        <Table>
            <TableHead>
                <TableRow>
                    <DataTableCell>
                        <strong>Name</strong>
                    </DataTableCell>
                    <DataTableCell>
                        <strong>Value</strong>
                    </DataTableCell>
                </TableRow>
            </TableHead>
            <TableBody>
                {Object.entries(userData).map(([key, value], i) => (
                    <TableRow key={i}>
                        <DataTableCell>{key}</DataTableCell>
                        <DataTableCell>{typeof value === "object" ? JSON.stringify(value) : value.toString()}</DataTableCell>
                    </TableRow>
                ))}
            </TableBody>
        </Table>
    );
}

export default ManageUser;
