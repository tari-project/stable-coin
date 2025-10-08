// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import * as React from "react";
import {useEffect} from "react";
import {StyledPaper} from "../../components/StyledComponents";
import {Alert, IconButton, Stack, Table, TableBody, TableCell, TableHead, TableRow, Typography} from "@mui/material";
import useTariProvider from "../../store/provider";
import {useNavigate} from "react-router-dom";
import {StableCoinIssuer} from "../../store/stableCoinIssuer";
import Box from "@mui/material/Box";
import {toHexString} from "@tari-project/tarijs-all";
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import KeyboardArrowLeftIcon from '@mui/icons-material/KeyboardArrowLeft';
import KeyboardArrowRightIcon from '@mui/icons-material/KeyboardArrowRight';

const INDEXER_ADDRESS = import.meta.env.VITE_INDEXER_ADDRESS;
const PAGE_SIZE = 10;

interface Props {
    issuer: StableCoinIssuer;
}

function TransactionList({issuer}: Props) {
    const {provider} = useTariProvider();
    const navigate = useNavigate();
    const [error, setError] = React.useState<Error | null>(null);
    const [isBusy, setBusy] = React.useState<boolean>(false);
    const [transactions, setTransactions] = React.useState<any[]>([]);
    const [page, setPage] = React.useState(0);

    if (!provider) {
        navigate("/");
        return <></>;
    }

    useEffect(() => {
        setBusy(true);
        query_transactions(page, PAGE_SIZE)
            .finally(() => setBusy(false));
    }, []);


    async function query_transactions(offset: number, limit: number) {
        const resourceAddress = issuer.vault.resourceAddress;
        console.log({resourceAddress});

        let res = await fetch(INDEXER_ADDRESS, {
            method: 'POST',

            headers: {
                "Content-Type": "application/json",
                "Accept": "application/json"
            },

            body: JSON.stringify({
                query: `{ getEvents(substateId: "${resourceAddress}", offset:${offset}, limit:${limit}) {substateId, templateAddress, txHash, topic, payload } }`,
                variables: {}
            })
        });

        let res_json = await res.json();
        console.log({res_json});
        let events = res_json.data.getEvents;

        let rows = events.map((event: any) => {
            return {
                tx_hash: toHexString(event.txHash),
                action: event.topic,
                vault_id: event.payload.vault_id,
                amount: event.payload.amount
            };
        });
        console.log({rows});
        setTransactions(rows);
    }

    function truncateText(text: string, length: number) {
        if (!length || !text || text.length <= length) {
            return text;
        }
        if (text.length <= length) {
            return text;
        }
        const leftChars = Math.ceil(length / 2);
        const rightChars = Math.floor(length / 2);
        return text.substring(0, leftChars) + '...' + text.substring(text.length - rightChars);
    }

    async function handleCopyClick(text: string) {
        if (text) {
            await navigator.clipboard.writeText(text);
        }
    }

    async function handleChangePage(newPage: number) {
        const offset = newPage * PAGE_SIZE;
        setBusy(true);
        try {
            await query_transactions(offset, PAGE_SIZE);
            setPage(newPage);
        } catch (err) {
            setError(err as Error);
        } finally {
            setBusy(false);
        }
    }

    return (
        <StyledPaper sx={{padding: 6}}>
            {error && (
                <Box sx={{paddingBottom: 4}}>
                    <Alert severity="error">{error.message}</Alert>
                </Box>
            )}
            {isBusy && (
                <Box sx={{paddingBottom: 4}}>
                    <Alert severity="info">Loading...</Alert>
                </Box>
            )}
            <Table sx={{minWidth: 650}} aria-label="simple table">
                <TableHead>
                    <TableRow>
                        <TableCell>Transaction Hash</TableCell>
                        <TableCell>Vault Id</TableCell>
                        <TableCell>Action</TableCell>
                        <TableCell>Amount</TableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {transactions.map((row) => (
                        <TableRow
                            sx={{'&:last-child td, &:last-child th': {border: 0}}}
                        >
                            <TableCell>
                                {truncateText(row.tx_hash, 20)}
                                <IconButton aria-label="copy" onClick={() => handleCopyClick(row.tx_hash)}>
                                    <ContentCopyIcon/>
                                </IconButton>
                            </TableCell>
                            <TableCell>
                                {truncateText(row.vault_id, 20)}
                                <IconButton aria-label="copy" onClick={() => handleCopyClick(row.vault_id)}>
                                    <ContentCopyIcon/>
                                </IconButton>
                            </TableCell>
                            <TableCell>{row.action}</TableCell>
                            <TableCell>{row.amount}</TableCell>
                        </TableRow>
                    ))}
                </TableBody>
            </Table>
            <Stack direction="row" justifyContent="right" spacing={2} alignItems="center">
                <IconButton aria-label="copy" onClick={() => handleChangePage(Math.max(page - 1, 0))}>
                    <KeyboardArrowLeftIcon/>
                </IconButton>
                <Typography sx={{}}>{page}</Typography>
                <IconButton aria-label="copy" onClick={() => handleChangePage(page + 1)}>
                    <KeyboardArrowRightIcon/>
                </IconButton>
            </Stack>
        </StyledPaper>
    );
}

export default TransactionList;
