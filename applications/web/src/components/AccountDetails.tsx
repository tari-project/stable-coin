// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

import { Account } from "@tariproject/tarijs";
import Box from "@mui/material/Box";

export function AccountDetails({ account }: { account: Account }) {
  return (
    <Box sx={{ paddingBottom: 4 }}>
      <h3>Account Details</h3>
      <p>
        <span>Account ID: {account.account_id}</span>
      </p>
      <p>
        <span>Address: {account.address}</span>
      </p>
      <p>
        <span>Public Key: {account.public_key}</span>
      </p>
      <p>
        Vaults: <br />
        {account.resources.map((r, i: number) => (
          <span key={i}>
              {r.type} Available: {r.balance} {("token_symbol" in r) ? r.token_symbol as string : r.resource_address}
            <br />
          </span>
        ))}
      </p>
    </Box>
  );
}
