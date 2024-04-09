// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::context::CliContext;
use crate::print_result::print_result;
use crate::transactions::StableCoinTransaction;
use crate::{transactions, value_parsers};
use clap::ArgAction;
use clap::Parser;
use tari_template_lib::crypto::RistrettoPublicKeyBytes;
use tari_template_lib::models::{Amount, ComponentAddress, Metadata, TemplateAddress};
use url::Url;

#[derive(Clone, Debug, clap::Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
    #[clap(flatten)]
    pub common: CommonCli,
}

impl Cli {
    pub fn init() -> Self {
        Self::parse()
    }
}

#[derive(Clone, Debug, clap::Parser)]
pub struct CommonCli {
    #[clap(
        short = 'u',
        long,
        default_value = "http://localhost:9000",
        env = "WALLET_URL"
    )]
    pub wallet_url: Url,
    #[clap(short = 'a', long, env = "FEE_ACCOUNT")]
    pub fee_account: Option<ComponentAddress>,
    #[clap(short = 't', long, env = "ISSUER_TEMPLATE")]
    pub issuer_template: TemplateAddress,
    #[clap(short = 'f', long, value_parser = value_parsers::amount, default_value = "2000")]
    pub max_fee: Amount,
}

#[derive(Clone, Debug, clap::Subcommand)]
pub enum Command {
    #[clap(subcommand)]
    Issuer(IssuerSubcommand),
}

#[derive(Clone, Debug, clap::Subcommand)]
pub enum IssuerSubcommand {
    Create(IssuerCreateSubcommand),
}

impl IssuerSubcommand {
    pub async fn run(self, mut context: CliContext) -> anyhow::Result<()> {
        match self {
            Self::Create(cmd) => {
                let params = context.get_build_params().await?;
                let transaction = transactions::build(params, cmd.into());
                let result = context.submit_and_wait_transaction(transaction).await?;
                print_result(&result)
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, clap::Args)]
pub struct IssuerCreateSubcommand {
    #[clap(value_parser = value_parsers::amount)]
    pub initial_token_supply: Amount,
    pub token_symbol: String,
    #[clap(value_parser = value_parsers::metadata)]
    pub token_metadata: Metadata,
    #[clap(long, short, action=ArgAction::SetFalse)]
    pub enable_wrapped_token: bool,
}

impl From<IssuerCreateSubcommand> for StableCoinTransaction {
    fn from(cmd: IssuerCreateSubcommand) -> Self {
        StableCoinTransaction::CreateIssuer {
            initial_token_supply: cmd.initial_token_supply,
            token_symbol: cmd.token_symbol,
            token_metadata: cmd.token_metadata,
            view_key: RistrettoPublicKeyBytes::default(),
            enable_wrapped_token: cmd.enable_wrapped_token,
        }
    }
}
