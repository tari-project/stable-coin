// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import Grid from "@mui/material/Grid";
import * as React from "react";
import { StyledPaper } from "../../components/StyledComponents.ts";
import { Alert, CircularProgress, TextField } from "@mui/material";
import Button from "@mui/material/Button";
import useTariProvider from "../../store/provider.ts";
import { useNavigate } from "react-router-dom";
import { ActiveIssuer } from "../../store/activeIssuer.ts";
import { SimpleTransactionResult } from "../../types.ts";
import Box from "@mui/material/Box";
import { useEffect } from "react";
import { types } from "@tariproject/tarijs";
import IconButton from "@mui/material/IconButton";
import { RefreshOutlined } from "@mui/icons-material";

interface Props {
  issuer: ActiveIssuer;
}

function WrappedToken({ issuer }: Props) {
  const [formValues, setFormValues] = React.useState({} as any);
  const { provider } = useTariProvider();
  const navigate = useNavigate();
  const [error, setError] = React.useState<Error | null>(null);
  const [invalid, setInvalid] = React.useState({} as any);
  const [isBusy, setIsBusy] = React.useState(false);
  const [account, setAccount] = React.useState<types.Account | null>(null);
  const [exchangeResult, setExchangeResult] = React.useState<SimpleTransactionResult | null>(null);

  if (!provider) {
    navigate("/");
    return <></>;
  }

  useEffect(() => {
    loadAccount();
  }, [issuer]);

  const set = (key: string) => (evt: React.ChangeEvent<HTMLInputElement>) => {
    if (!evt.target.validity.valid) {
      return;
    }
    setFormValues({ ...formValues, [key]: evt.target.value });
  };
  const onValidate = (key: string) => (evt: React.FocusEvent<HTMLInputElement>) => {
    console.log(evt.target.validity);
    if (evt.target.validity.valid) {
      delete invalid[key];
      setInvalid({ ...invalid });
    } else {
      setInvalid({ [key]: `Invalid` });
      return;
    }
  };

  const submitExchangeTokenForWrapped = async () => {
    setError(null);
    try {
      setIsBusy(true);
      const result = await provider.exchangeStableForWrappedToken(
        issuer.id,
        account!.address,
        issuer.vault.resourceAddress,
        issuer.userAuthResource,
        Number(formValues.userId.trim()),
        Number(formValues.exchangeAmount.trim()),
      );
      if (result.accept) {
        setExchangeResult(result);
      } else {
        setError(new Error(JSON.stringify(result.rejectReason)));
      }

      loadAccount();
    } catch (e) {
      console.error(e);
      setError(e as Error);
    } finally {
      setIsBusy(false);
    }
  };

  const submitExchangeWrappedForToken = async () => {
    setError(null);
    try {
      setIsBusy(true);
      const result = await provider.exchangeWrappedForStable(
        issuer.id,
        account!.address,
        issuer.wrappedToken!.resource,
        issuer.userAuthResource,
        Number(formValues.userId.trim()),
        Number(formValues.exchangeAmount.trim()),
      );
      if (result.accept) {
        setExchangeResult(result);
      } else {
        setError(new Error(JSON.stringify(result.rejectReason)));
      }

      loadAccount();
    } catch (e) {
      console.error(e);
      setError(e as Error);
    } finally {
      setIsBusy(false);
    }
  };

  const loadAccount = () => {
    setIsBusy(true);
    setAccount(null);
    provider
      .getAccount()
      .then((account) => {
        setAccount(account);
      })
      .catch((e) => setError(e))
      .finally(() => setIsBusy(false));
  };

  return (
    <StyledPaper sx={{ padding: 6 }}>
      {error && (
        <Box sx={{ paddingBottom: 4 }}>
          <Alert severity="error">{error.message}</Alert>
        </Box>
      )}
      {exchangeResult?.accept && (
        <Box sx={{ paddingBottom: 4 }}>
          <Alert severity="success">
            {`Transaction ${exchangeResult.transactionId} was accepted`}
          </Alert>
        </Box>
      )}
      <Box sx={{ paddingBottom: 4 }}>
        NOTE: this functionality demonstrates a wrapped token exchange that a user would use when connecting their
        wallet (not the issuer's wallet). Switch the default account to one that has tokens to exchange.
      </Box>
      <Grid container spacing={2}>
        <Grid container spacing={2} sx={{ paddingBottom: 2 }}>
          <Grid item xs={12} md={12} lg={12} sx={{ textAlign: "left" }}>
            <IconButton onClick={loadAccount}>
              <RefreshOutlined />
            </IconButton>
            {account ? <AccountDetails account={account} /> : <CircularProgress size={24} />}
          </Grid>
          <Grid item xs={12} md={12} lg={12} sx={{ textAlign: "left" }}>
            <TextField
              label="User Id"
              value={formValues.userId || ""}
              required
              inputProps={{ pattern: "[0-9]+" }}
              onChange={set("userId")}
              onBlur={onValidate("userId")}
            />
          </Grid>
          <Grid item xs={12} md={12} lg={12} sx={{ textAlign: "left" }}>
            <TextField
              label="Exchange Amount"
              value={formValues.exchangeAmount || ""}
              required
              inputProps={{ pattern: "[0-9]+" }}
              onChange={set("exchangeAmount")}
              onBlur={onValidate("exchangeAmount")}
            />
          </Grid>
        </Grid>
        <Grid container spacing={2}>
          <Grid item xs={6} md={6} lg={6} sx={{ textAlign: "left" }}>
            <Button
              variant="contained"
              color="primary"
              disabled={!formValues.exchangeAmount || Boolean(invalid.exchangeAmount) || isBusy}
              onClick={() => submitExchangeTokenForWrapped()}
            >
              {isBusy ? <CircularProgress size={24} /> : <span>Exchange Stable -&gt; Wrapped</span>}
            </Button>
          </Grid>
          <Grid item xs={6} md={6} lg={6} sx={{ textAlign: "left" }}>
            <Button
              variant="contained"
              color="primary"
              disabled={!formValues.exchangeAmount || Boolean(invalid.exchangeAmount) || isBusy}
              onClick={() => submitExchangeWrappedForToken()}
            >
              {isBusy ? <CircularProgress size={24} /> : <span>Exchange Wrapped -&gt; Stable</span>}
            </Button>
          </Grid>
          {invalid.exchangeAmount && (
            <Grid item xs={12} md={12} lg={12} sx={{ textAlign: "left" }}>
              <Alert severity="error">{invalid.exchangeAmount}</Alert>
            </Grid>
          )}
        </Grid>
      </Grid>
    </StyledPaper>
  );
}

function AccountDetails({ account }: { account: types.Account }) {
  return (
    <Box sx={{ paddingBottom: 4 }}>
      <h3>Account Details</h3>
      <p>
        <span>Account ID:</span>
        <span>{account.account_id}</span>
      </p>
      <p>
        <span>Address: </span>
        <span>{account.address}</span>
      </p>
      <p>
        <span>Public Key: </span>
        <span>{account.public_key}</span>
      </p>
      <p>
        <span>Vaults: </span>
        {account.resources.map((r, i: number) => (
          <p key={i}>
            <span>
              {r.type} Available: {r.balance} {("token_symbol" in r) ? r.token_symbol as string : r.resource_address}
            </span>
          </p>
        ))}
      </p>
    </Box>
  );
}

export default WrappedToken;
