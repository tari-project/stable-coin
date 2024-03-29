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
import * as React from "react";
import useTariProvider from "../../store/provider.ts";
import { Alert, TextField } from "@mui/material";
import Button from "@mui/material/Button";
import { ComponentAddress, ResourceAddress } from "@tariproject/typescript-bindings";
import { SimpleTransactionResult, splitOnce } from "../../types.ts";

interface Props {
  issuerId: ComponentAddress;
  userId: number;
  userBadge: ResourceAddress;
  adminAuthBadge: ResourceAddress;
  badgeData: object;
  badgeMutableData: object;
  onChange?: (result: SimpleTransactionResult) => void;
}

function RecallTokens(props: Props) {
  const { provider } = useTariProvider();

  const [isBusy, setIsBusy] = React.useState(false);
  const [error, setError] = React.useState<Error | null>(null);
  const [success, setSuccess] = React.useState<string | null>(null);
  const [recallAmount, setRecallAmount] = React.useState<number>(0);

  const userAccount = props.badgeData.user_account!;
  const [userBadgeResource, _nft] = splitOnce(props.userBadge, " ")!;

  const handleOnRecall = async (e) => {
    e.preventDefault();
    setIsBusy(true);
    setSuccess(null);
    setError(null);

    try {
      const result = await provider.recallTokens(
        props.issuerId,
        props.adminAuthBadge,
        userAccount,
        userBadgeResource,
        props.userId,
        recallAmount,
      );
      if (result.accept) {
        props.onChange?.(result);
        setSuccess(`User tokens recalled in transaction ${result.transactionId}`);
      }

      throw new Error(`Transaction failed ${JSON.stringify(result.rejectReason)}`);
    } catch (e) {
      setError(e);
    } finally {
      setIsBusy(false);
    }
  };

  return (
    <Grid container spacing={2} sx={{ textAlign: "left", paddingY: 4 }}>
      <Grid item xs={12} md={12} lg={12}>
        <h2>Recall</h2>
      </Grid>
      <Grid item xs={12} md={12} lg={12}>
        <form onSubmit={handleOnRecall}>
          <Grid container spacing={2}>
            <Grid item xs={4} md={4} lg={4}>
              <TextField
                name="recall_amount"
                placeholder="Recall amount"
                label="Recall amount"
                fullWidth
                required
                type="number"
                value={recallAmount}
                onChange={(e) => setRecallAmount(parseInt(e.target.value))}
              />
            </Grid>
            <Grid item xs={4} md={4} lg={4}>
              <Button variant="contained" type="submit" disabled={isBusy || !recallAmount} color="secondary">
                Recall
              </Button>
            </Grid>
          </Grid>
        </form>
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
    </Grid>
  );
}

export default RecallTokens;
