// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::CommonCli;
use crate::transactions::BuildParams;
use tari_template_lib::models::{Amount, TemplateAddress};
use tari_transaction::UnsignedTransaction;
use tari_wallet_daemon_client::types::{
    AuthLoginAcceptRequest, AuthLoginRequest, TransactionSubmitRequest,
    TransactionWaitResultRequest, TransactionWaitResultResponse,
};
use tari_wallet_daemon_client::WalletDaemonClient;

pub struct CliContext {
    client: Option<WalletDaemonClient>,
    common_cli: CommonCli,
}

impl CliContext {
    pub fn new(common_cli: CommonCli) -> Self {
        Self {
            client: None,
            common_cli,
        }
    }

    pub fn issuer_template(&self) -> TemplateAddress {
        self.common_cli.issuer_template
    }

    pub fn max_fee(&self) -> Amount {
        self.common_cli.max_fee
    }

    pub async fn connect_wallet_client(&mut self) -> anyhow::Result<&mut WalletDaemonClient> {
        if let Some(ref mut client_mut) = self.client {
            return Ok(client_mut);
        }

        let mut client = WalletDaemonClient::connect(self.common_cli.wallet_url.clone(), None)?;

        // "Log in" as admin (TODO: exchange secret credentials for auth token)
        let resp = client
            .auth_request(AuthLoginRequest {
                permissions: vec!["Admin".to_string()],
                duration: None,
            })
            .await?;

        let resp = client
            .auth_accept(AuthLoginAcceptRequest {
                auth_token: resp.auth_token,
                name: "cli".to_string(),
            })
            .await?;

        client.set_auth_token(resp.permissions_token);

        self.client = Some(client);
        Ok(self.client.as_mut().unwrap())
    }

    pub(crate) async fn get_build_params(&mut self) -> anyhow::Result<BuildParams> {
        let fee_account = match self.common_cli.fee_account {
            Some(fee_account) => fee_account,
            None => {
                let client_mut = self.connect_wallet_client().await?;
                let resp = client_mut.accounts_get_default().await?;
                resp.account.address.as_component_address().unwrap()
            }
        };

        Ok(BuildParams {
            fee_account,
            issuer_template: self.issuer_template(),
            max_fee: self.max_fee(),
        })
    }

    pub(crate) async fn submit_and_wait_transaction(
        &mut self,
        transaction: UnsignedTransaction,
    ) -> anyhow::Result<TransactionWaitResultResponse> {
        let fee_account = self.common_cli.fee_account;
        let client_mut = self.connect_wallet_client().await?;

        let resp = match fee_account {
            Some(fee_account) => client_mut.accounts_get(fee_account.into()).await?,
            None => client_mut.accounts_get_default().await?,
        };

        let resp = client_mut
            .submit_transaction(TransactionSubmitRequest {
                transaction: Some(transaction),
                signing_key_index: Some(resp.account.key_index),
                ..Default::default()
            })
            .await?;
        let resp = client_mut
            .wait_transaction_result(TransactionWaitResultRequest {
                transaction_id: resp.transaction_id,
                timeout_secs: None,
            })
            .await?;

        Ok(resp)
    }
}
