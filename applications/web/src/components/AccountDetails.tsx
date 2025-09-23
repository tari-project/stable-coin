// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import {AccountData} from "@tari-project/tarijs-all";
import {substateIdToString} from "@tari-project/typescript-bindings";
import {Grid2 as Grid, Table, TableBody, TableContainer, TableRow} from "@mui/material";
import {DataTableCell} from "./StyledComponents";

export function AccountDetails({account}: { account: AccountData }) {
    return (
        <Grid container sx={{paddingBottom: 4}}>
            <Grid size={12}>
                <h3>Account Details</h3>
                <TableContainer>
                    <Table>
                        <TableBody>
                            <TableRow>
                                <DataTableCell width={90} sx={{borderBottom: "none", textAlign: "center"}}>
                                    Account Address:
                                </DataTableCell>
                                <DataTableCell>
                                    {substateIdToString(account.component_address)}
                                </DataTableCell>
                            </TableRow>
                            <TableRow>
                                <DataTableCell width={90} sx={{borderBottom: "none", textAlign: "center"}}>
                                    Wallet Address:
                                </DataTableCell>
                                <DataTableCell>
                                    {account.wallet_address}
                                </DataTableCell>
                            </TableRow>
                            <TableRow>
                                <DataTableCell width={90} sx={{borderBottom: "none", textAlign: "center"}}>
                                    Vaults
                                </DataTableCell>
                                <DataTableCell></DataTableCell>
                            </TableRow>

                            {account.vaults.map((r, i: number) => (
                                <TableRow key={i}>
                                    <DataTableCell width={90} sx={{borderBottom: "none", textAlign: "center"}}>
                                        {r.type}
                                    </DataTableCell>
                                    <DataTableCell>
                                        {r.balance || (("token_ids" in r && Array.isArray(r.token_ids)) ? r.token_ids.length : "")} {r.token_symbol || r.resource_address}
                                    </DataTableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </TableContainer>
            </Grid>
        </Grid>
    );
}
