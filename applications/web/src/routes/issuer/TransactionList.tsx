// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import * as React from "react";
import { StyledPaper } from "../../components/StyledComponents.ts";
import { Alert } from "@mui/material";
import useTariProvider from "../../store/provider.ts";
import { useNavigate } from "react-router-dom";
import { StableCoinIssuer } from "../../store/stableCoinIssuer.ts";
import Box from "@mui/material/Box";
import { useEffect } from "react";
import { utils } from "@tariproject/tarijs";

const INDEXER_ADDRESS = import.meta.env.VITE_INDEXER_ADDRESS;

interface Props {
  issuer: StableCoinIssuer;
}

function TransactionList({ issuer }: Props) {
  const { provider } = useTariProvider();
  const navigate = useNavigate();
  const [error, setError] = React.useState<Error | null>(null);
  const [invalid, setInvalid] = React.useState<any>({});
  const [busy, setBusy] = React.useState<{ [key: string]: boolean }>({});

  if (!provider) {
    navigate("/");
    return <></>;
  }

  useEffect(() => {
    query_transactions();
  }, []);

  const isBusy = Object.keys(busy).length > 0;

  async function query_transactions() {
    const resourceAddress = issuer.vault.resourceAddress;
    console.log({resourceAddress});

    let res = await fetch(INDEXER_ADDRESS, {
      method: 'POST',

      headers: {
        "Content-Type": "application/json",
        "Accept": "application/json"
      },

      body: JSON.stringify({
        query: `{ getEvents(substateId: "${resourceAddress}", offset:0, limit:10) {substateId, templateAddress, txHash, topic, payload } }`,
        variables: {}
      })
    });
    let res_json = await res.json();
    console.log({ res_json });
    let events = res_json.data.getEvents;
    events.forEach((event) => {
      console.log(utils.toHexString(event.txHash));
    });
  }

  return (
    <StyledPaper sx={{ padding: 6 }}>
      {error && (
        <Box sx={{ paddingBottom: 4 }}>
          <Alert severity="error">{error.message}</Alert>
        </Box>
      )}            
    </StyledPaper>
  );
}

export default TransactionList;
