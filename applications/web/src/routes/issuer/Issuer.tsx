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
import SecondaryHeading from "../../components/SecondaryHeading.tsx";
import { StyledPaper } from "../../components/StyledComponents.ts";
import * as React from "react";
import useTariProvider from "../../store/provider.ts";
import useActiveIssuer, { ActiveIssuer } from "../../store/activeIssuer.ts";
import { Alert, CircularProgress } from "@mui/material";
import Button from "@mui/material/Button";
import { useNavigate, useParams } from "react-router-dom";
import { useEffect } from "react";
import * as cbor from "../../cbor";
import SupplyControl from "./SupplyControl.tsx";
import { SimpleTransactionResult } from "../../types.ts";
import Transfers from "./Transfers.tsx";
import WrappedToken from "./WrappedToken.tsx";
import { CborValue } from "../../cbor";

interface IssuerDetailsProps {
  issuer: ActiveIssuer;
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <Grid container item xs={12} md={12} lg={12}>
      <Grid item xs={3} md={3} lg={3} sx={{ textAlign: "left" }}>
        {label}
      </Grid>
      <Grid item xs={9} md={9} lg={9} sx={{ textAlign: "left" }}>
        {value}
      </Grid>
    </Grid>
  );
}

function IssuerDetails({ issuer }: IssuerDetailsProps) {
  const navigate = useNavigate();
  const params = useParams();
  return (
    <StyledPaper sx={{ padding: 6 }}>
      <Grid container spacing={2} sx={{ paddingBottom: 4 }}>
        <Detail label="Component ID" value={issuer.id} />
        <Detail label="Resource Address" value={issuer.vault.resourceAddress} />
        <Detail label="Revealed Vault Supply" value={issuer.vault.revealedAmount.toString()} />
        <Detail label="Admin Badge Resource" value={issuer.adminAuthResource} />
        <Detail label="User Badge Resource" value={issuer.userAuthResource} />
        <Detail label="Wrapped Token" value={issuer.wrappedToken?.resource || "Disabled"} />
        <Detail label="Wrapped Balance" value={issuer.wrappedToken?.balance.toString() || "N/A"} />
      </Grid>
      <Grid container spacing={2} sx={{ textAlign: "left" }}>
        <Button variant="contained" color="secondary" onClick={() => navigate(`/issuers/${params.issuerId}/users`)}>
          Manage Users
        </Button>
      </Grid>
    </StyledPaper>
  );
}


function Issuer() {
  const { provider } = useTariProvider();
  const [isLoading, setIsLoading] = React.useState(false);
  const { activeIssuer, setActiveIssuer } = useActiveIssuer();
  const [error, setError] = React.useState<Error | null>(null);
  const [success, setSuccess] = React.useState<string | null>(null);
  const params = useParams();
  const navigate = useNavigate();

  if (!provider) {
    useEffect(() => {
      navigate("/");
    }, []);
    return <></>;
  }

  function load() {
    if (!isLoading && !activeIssuer && !error) {
      setIsLoading(true);
      const getComponent = provider!.getSubstate(params.issuerId!).then(async (issuer) => {
        const { substate } = issuer.value;
        const { substate_id, version } = (issuer as any).record;
        if (!("Component" in substate)) {
          throw new Error("Issuer not component");
        }
        const structMap = substate.Component.body.state as CborValue;
        const vaultId = cbor.getValueByPath(structMap, "$.token_vault");
        const adminAuthResource = cbor.getValueByPath(structMap, "$.admin_auth_resource");
        const userAuthResource = cbor.getValueByPath(structMap, "$.user_auth_resource");
        const vault = await provider!.getSubstate(vaultId);
        if (!("Vault" in vault.value.substate)) {
          throw new Error("Substate is not a vault");
        }
        const container = vault.value.substate.Vault.resource_container;
        if (!("Confidential" in container)) {
          throw new Error("Vault is not confidential");
        }

        const wrappedToken = cbor.getValueByPath(structMap, "$.wrapped_token");
        const wrappedVault = wrappedToken ? await provider!.getSubstate(wrappedToken.vault) : null;
        const wrappedContainer = (wrappedVault?.value.substate as any).Vault.resource_container.Fungible;

        setActiveIssuer({
          id: substate_id.Component,
          version,
          vault: {
            id: vaultId,
            resourceAddress: container.Confidential.address,
            revealedAmount: container.Confidential.revealed_amount,
          },
          adminAuthResource,
          userAuthResource,
          wrappedToken: {
            resource: wrappedContainer.address,
            balance: wrappedContainer.amount,
            ...wrappedToken,
          },
        } as ActiveIssuer);
      });

      Promise.allSettled([getComponent])
        .then((results) => {
          if (results[0].status === "rejected") {
            setError(new Error(results[0].reason));
          } else {
            setError(null);
          }
        })
        .catch((err) => {
          setError(err);
        })
        .finally(() => setIsLoading(false));
    }
  }

  React.useEffect(load, [isLoading, activeIssuer, error]);

  function handleTransactionResult(result: SimpleTransactionResult) {
    if (result.accept) {
      setSuccess(`Transaction ${result.transactionId} succeeded`);
      setActiveIssuer(null);
      load();
    }

    const reject = result.onlyFeeAccepted?.[1] || result.rejected;
    if (reject) {
      setSuccess(null);
      setError(new Error(`Transaction rejected: ${JSON.stringify(reject)}`));
    }
  }

  return (
    <>
      <Grid item sm={12} md={12} xs={12}>
        <SecondaryHeading>Issuer</SecondaryHeading>
      </Grid>
      {error && (
        <Grid item xs={12} md={12} lg={12}>
          <Alert severity="error">{error.message}</Alert>
        </Grid>
      )}
      {success && (
        <Grid item xs={12} md={12} lg={12}>
          <Alert severity="success">{success}</Alert>
        </Grid>
      )}
      {isLoading || !activeIssuer ? (
        <CircularProgress />
      ) : (
        <>
          <Grid item xs={12} md={12} lg={12}>
            <h2>Details</h2>
            <IssuerDetails issuer={activeIssuer} />
          </Grid>
          <Grid item xs={12} md={12} lg={12}>
            <h2>Supply</h2>
            <SupplyControl
              issuer={activeIssuer}
              onTransactionSubmit={() => setSuccess(null)}
              onTransactionResult={handleTransactionResult}
            />
          </Grid>
          <Grid item xs={12} md={12} lg={12}>
            <h2>Transfers</h2>
            <Transfers issuer={activeIssuer} onTransactionResult={handleTransactionResult} />
          </Grid>
          {activeIssuer.wrappedToken && (
            <Grid item xs={12} md={12} lg={12}>
              <h2>Wrapped Token</h2>
              <WrappedToken issuer={activeIssuer} />
            </Grid>
          )}
        </>
      )}
    </>
  );
}

export default Issuer;
