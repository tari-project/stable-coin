// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::args;
use tari_template_lib::crypto::RistrettoPublicKeyBytes;
use tari_template_lib::models::{Amount, Metadata, TemplateAddress};
use tari_template_lib::prelude::ComponentAddress;
use tari_transaction::{Transaction, UnsignedTransaction};

pub enum StableCoinTransaction {
    CreateIssuer {
        initial_token_supply: Amount,
        token_symbol: String,
        token_metadata: Metadata,
        view_key: RistrettoPublicKeyBytes,
        enable_wrapped_token: bool,
    },
}

pub struct BuildParams {
    pub issuer_template: TemplateAddress,
    pub fee_account: ComponentAddress,
    pub max_fee: Amount,
}

pub fn build(params: BuildParams, st_transaction: StableCoinTransaction) -> UnsignedTransaction {
    match st_transaction {
        StableCoinTransaction::CreateIssuer {
            initial_token_supply,
            token_symbol,
            token_metadata,
            view_key,
            enable_wrapped_token,
        } => create_issuer(
            params,
            initial_token_supply,
            token_symbol,
            token_metadata,
            view_key,
            enable_wrapped_token,
        ),
    }
}

fn create_issuer(
    params: BuildParams,
    initial_token_supply: Amount,
    token_symbol: String,
    token_metadata: Metadata,
    view_key: RistrettoPublicKeyBytes,
    enable_wrapped_token: bool,
) -> UnsignedTransaction {
    Transaction::builder()
        .fee_transaction_pay_from_component(params.fee_account, params.max_fee)
        .call_function(
            params.issuer_template,
            "instantiate",
            args!(
                initial_token_supply,
                token_symbol,
                token_metadata,
                view_key,
                enable_wrapped_token
            ),
        )
        .put_last_instruction_output_on_workspace("issuer_badge")
        .call_method(
            params.fee_account,
            "deposit",
            args!(Workspace("issuer_badge")),
        )
        .build_unsigned_transaction()
}
