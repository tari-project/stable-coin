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

import {Vault, VaultId} from "@tariproject/typescript-bindings";
import useTariProvider from "../../store/provider.ts";
import {providers} from "@tariproject/tarijs";
import {useEffect} from "react";
import {Alert, CircularProgress, Table, TableBody, TableContainer, TableHead, TableRow} from "@mui/material";
import {DataTableCell} from "../../components/StyledComponents.ts";

const {
    VaultBalances
} = providers;

interface Props {
    vaultId: VaultId,
    vault: Vault
}

function UserVault(props: Props) {
    const {provider} = useTariProvider();
    const [balances, setBalances] = React.useState<VaultBalances | null>(null);
    const [error, setError] = React.useState<Error | null>(null);
    const [loading, setLoading] = React.useState<boolean>(false);
    const [total, setTotal] = React.useState<number>(0);
    const [revealedAmount, setRevealedAmount] = React.useState<number>(0);

    useEffect(() => {
        setLoading(true);
        provider.getConfidentialVaultBalance(props.vaultId)
            .then((balances) => {
                setBalances(balances);
                let revealedAmount = props.vault.resource_container.Confidential?.revealed_amount || 0;
                const total = Object.values(balances.balances).reduce((acc, key) => acc + (key || 0), 0) + revealedAmount;
                setRevealedAmount(revealedAmount);
                setTotal(total);
            })
            .catch((err) => {
                setError(err);
            })
            .finally(() => {
                setLoading(false);
            });
    }, [props.vaultId]);

    if (loading) {
        return <CircularProgress/>;
    }

    if (error) {
        return <Alert severity="error">{error.message}</Alert>;
    }

    if (!balances) {
        return <Alert severity="info">No balances found</Alert>;
    }


    return (
        <TableContainer>
            <Table>
                <TableHead>
                    <TableRow>
                        <DataTableCell><strong>UTXO</strong></DataTableCell>
                        <DataTableCell><strong>Balance</strong></DataTableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {Object.keys(balances.balances).map((commitment, i) => (
                        <TableRow key={i}>
                            <DataTableCell><span title={commitment}>UTXO {i}</span></DataTableCell>
                            <DataTableCell>{balances.balances[commitment]}</DataTableCell>
                        </TableRow>
                    ))}
                    <TableRow>
                        <DataTableCell><strong>Revealed Balance</strong></DataTableCell>
                        <DataTableCell>{revealedAmount}</DataTableCell>
                    </TableRow>
                    <TableRow>
                        <DataTableCell><strong>Total</strong></DataTableCell>
                        <DataTableCell>{total}</DataTableCell>
                    </TableRow>
                </TableBody>
            </Table>
        </TableContainer>
    );
}


export default UserVault;
