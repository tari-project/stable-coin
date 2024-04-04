// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import { Account } from "@tariproject/tarijs";
import { Table, TableBody, TableContainer, TableRow } from "@mui/material";
import { DataTableCell } from "./StyledComponents.ts";
import Grid from "@mui/material/Grid";

export function AccountDetails({ account }: { account: Account }) {
  return (
    <Grid container sx={{ paddingBottom: 4 }}>
      <Grid item xs={12}>
        <h3>Account Details</h3>
        <TableContainer>
          <Table>
            <TableBody>
              <TableRow>
                <DataTableCell width={90} sx={{ borderBottom: "none", textAlign: "center" }}>
                  Account Address:
                </DataTableCell>
                <DataTableCell>
                  {account.address}
                </DataTableCell>
              </TableRow>
              <TableRow>
                <DataTableCell width={90} sx={{ borderBottom: "none", textAlign: "center" }}>
                  Account Public Key:
                </DataTableCell>
                <DataTableCell>
                  {account.public_key}
                </DataTableCell>
              </TableRow>
              <TableRow>
                <DataTableCell width={90} sx={{ borderBottom: "none", textAlign: "center" }}>
                  Vaults
                </DataTableCell>
                <DataTableCell></DataTableCell>
              </TableRow>

              {account.resources.map((r, i: number) => (
                <TableRow key={i}>
                  <DataTableCell width={90} sx={{ borderBottom: "none", textAlign: "center" }}>
                    {r.type}
                  </DataTableCell>
                  <DataTableCell>
                    {r.balance || (("token_ids" in r && Array.isArray(r.token_ids)) ? r.token_ids.length : "")} {("token_symbol" in r) ? r.token_symbol as string : r.resource_address}
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
