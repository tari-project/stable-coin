//! Proof-of-concept operator tool for the `issuer-minimal` stable coin template.
//!
//! It talks to an Ootle indexer directly through `ootle-rs`: no wallet daemon is involved. Role
//! keys live in passphrase-encrypted key files.

// The template crate is a `cdylib`, so its role and config types are pulled in by path.
#[path = "../../../templates/private_stable_coin/issuer-minimal/src/config.rs"]
mod config;
mod issuer;
mod keystore;
#[allow(dead_code)]
#[path = "../../../templates/private_stable_coin/issuer-minimal/src/roles.rs"]
mod roles;
mod web;

use std::{fs, path::PathBuf};

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use ootle_rs::{Network, default_indexer_url, template_types::Amount};
use tari_ootle_common_types::engine_types::published_template::PublishedTemplateAddress;

use crate::{
    issuer::{Action, DeployParams, Deployment, Issuer, TxSummary, parse_holder},
    roles::RoleConfig,
};

#[derive(Parser)]
#[command(
    name = "stablecoin",
    version,
    about = "Operate an issuer-minimal stable coin (proof of concept)"
)]
struct Cli {
    /// The encrypted key file that signs transactions.
    #[arg(
        long,
        short = 'k',
        global = true,
        env = "STABLECOIN_KEY",
        default_value = "issuer.key"
    )]
    key: PathBuf,
    /// The deployment file written by `deploy`.
    #[arg(
        long,
        short = 'd',
        global = true,
        env = "STABLECOIN_DEPLOYMENT",
        default_value = "deployment.json"
    )]
    deployment: PathBuf,
    /// Indexer URL. Defaults to the deployment's indexer, or the network's public indexer.
    #[arg(long, global = true, env = "STABLECOIN_INDEXER")]
    indexer: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create or inspect a key file.
    #[command(subcommand)]
    Key(KeyCommand),
    /// Take test funds from the network faucet (test networks only).
    Faucet,
    /// Publish a template binary and print its address.
    Publish {
        /// Path to the compiled template WASM.
        wasm: PathBuf,
    },
    /// Instantiate a coin from a published template.
    Deploy(Box<DeployArgs>),
    /// Show the coin's supply, roles and pause state.
    Status,
    /// Mint new supply into the issuer vault (minter).
    Mint { amount: u64 },
    /// Burn supply from the issuer vault (burner).
    Burn { amount: u64 },
    /// Send coins from the issuer vault to an address (treasurer).
    Send { to: String, amount: u64 },
    /// Pause issuance and treasury movements (pauser).
    Pause,
    /// Unpause (governor).
    Unpause,
    /// Give a role to another key (governor).
    SetRole {
        /// governor, minter, burner, treasurer, compliance or pauser.
        role: String,
        /// The new holder: an Ootle address or a hex public key.
        holder: String,
    },
    /// Propose the next governor (governor).
    ProposeGovernor { holder: String },
    /// Confirm a governor proposal naming this key.
    ConfirmGovernor,
    /// Withdraw the pending governor proposal (governor).
    CancelGovernor,
    /// Set or remove the supply cap (governor).
    SetMaxSupply {
        /// The cap; omit to remove it.
        max_supply: Option<u64>,
    },
    /// Pull a revealed amount out of an account into the issuer vault (compliance).
    Recall { from: String, amount: u64 },
    /// Freeze UTXOs by hex id (compliance).
    Freeze { utxos: Vec<String> },
    /// Unfreeze UTXOs (compliance).
    Unfreeze { utxos: Vec<String> },
    /// Serve the local web dashboard.
    Serve {
        #[arg(long, default_value_t = 7400)]
        port: u16,
    },
}

#[derive(Subcommand)]
enum KeyCommand {
    /// Create a new key file.
    New {
        #[arg(long, default_value = "localnet")]
        network: Network,
    },
    /// Print the address recorded in the key file.
    Show,
}

#[derive(Args)]
struct DeployArgs {
    /// The published template address.
    #[arg(long)]
    template: PublishedTemplateAddress,
    #[arg(long, default_value = "STC")]
    symbol: String,
    #[arg(long, default_value = "Stable Coin Inc.")]
    name: String,
    #[arg(long, default_value_t = 1_000_000_000)]
    initial_supply: u64,
    #[arg(long, default_value_t = 2)]
    divisibility: u8,
    #[arg(long)]
    max_supply: Option<u64>,
    /// Role holders (address or hex public key). The governor defaults to this key, and unset
    /// roles follow the governor.
    #[arg(long)]
    governor: Option<String>,
    #[arg(long)]
    minter: Option<String>,
    #[arg(long)]
    burner: Option<String>,
    #[arg(long)]
    treasurer: Option<String>,
    #[arg(long)]
    compliance: Option<String>,
    #[arg(long)]
    pauser: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Command::Key(command) = &cli.command {
        return key_command(&cli, command);
    }

    let key = keystore::open(&cli.key, &keystore::passphrase(false)?)?;
    let deployment = Deployment::load(&cli.deployment);
    let indexer = match (&cli.indexer, &deployment) {
        (Some(url), _) => url.clone(),
        (None, Ok(deployment)) => deployment.indexer_url.clone(),
        (None, Err(_)) => default_indexer_url(key.network()).to_string(),
    };
    let mut issuer = Issuer::connect(key, &indexer).await?;

    match cli.command {
        Command::Key(_) => unreachable!("handled above"),
        Command::Faucet => print_tx("Took faucet funds", &issuer.faucet().await?),
        Command::Publish { wasm } => {
            let binary =
                fs::read(&wasm).with_context(|| format!("Failed to read {}", wasm.display()))?;
            let (template, summary) = issuer.publish_template(binary).await?;
            print_tx("Published template", &summary);
            println!("Template:    {template}");
        }
        Command::Deploy(args) => {
            let rule = |holder: &Option<String>| holder.as_deref().map(parse_holder).transpose();
            let roles = RoleConfig {
                governor: rule(&args.governor)?.unwrap_or_else(|| issuer.signer_rule()),
                minter: rule(&args.minter)?,
                burner: rule(&args.burner)?,
                treasurer: rule(&args.treasurer)?,
                compliance: rule(&args.compliance)?,
                pauser: rule(&args.pauser)?,
            };
            let (deployment, summary) = issuer
                .deploy(DeployParams {
                    template: args.template.as_template_address(),
                    initial_supply: Amount::from(args.initial_supply),
                    symbol: args.symbol,
                    name: args.name,
                    divisibility: args.divisibility,
                    max_supply: args.max_supply.map(Amount::from),
                    roles,
                })
                .await?;
            deployment.save(&cli.deployment)?;
            print_tx("Deployed", &summary);
            println!("Component:   {}", deployment.component);
            println!("Resource:    {}", deployment.resource);
            println!("Saved to     {}", cli.deployment.display());
        }
        Command::Status => print_status(&issuer.status(&deployment?).await?),
        Command::Serve { port } => web::serve(issuer, deployment?, port).await?,
        command => {
            let (label, summary) = issuer.execute(&deployment?, action(command)).await?;
            print_tx(&label, &summary);
        }
    }
    Ok(())
}

fn action(command: Command) -> Action {
    match command {
        Command::Mint { amount } => Action::Mint { amount },
        Command::Burn { amount } => Action::Burn { amount },
        Command::Send { to, amount } => Action::Send { to, amount },
        Command::Recall { from, amount } => Action::Recall { from, amount },
        Command::Freeze { utxos } => Action::Freeze { utxos },
        Command::Unfreeze { utxos } => Action::Unfreeze { utxos },
        Command::Pause => Action::Pause,
        Command::Unpause => Action::Unpause,
        Command::SetRole { role, holder } => Action::SetRole { role, holder },
        Command::ProposeGovernor { holder } => Action::ProposeGovernor { holder },
        Command::ConfirmGovernor => Action::ConfirmGovernor,
        Command::CancelGovernor => Action::CancelGovernor,
        Command::SetMaxSupply { max_supply } => Action::SetMaxSupply { max_supply },
        Command::Key(_)
        | Command::Faucet
        | Command::Publish { .. }
        | Command::Deploy(_)
        | Command::Status
        | Command::Serve { .. } => unreachable!("handled by the caller"),
    }
}

fn key_command(cli: &Cli, command: &KeyCommand) -> anyhow::Result<()> {
    match command {
        KeyCommand::New { network } => {
            let key = keystore::create(&cli.key, *network, &keystore::passphrase(true)?)?;
            println!("Created {}", cli.key.display());
            println!("Address:     {}", key.to_address());
            println!("Public key:  {}", key.to_address().account_public_key());
            println!("Send public TARI to the address so this key can pay fees.");
        }
        KeyCommand::Show => println!("{}", keystore::recorded_address(&cli.key)?),
    }
    Ok(())
}

fn print_tx(label: &str, summary: &TxSummary) {
    println!("{label}");
    println!("Transaction: {}", summary.transaction_id);
    println!("Fee:         {}", summary.fee);
    for event in &summary.events {
        println!("Event:       {event}");
    }
}

fn print_status(status: &issuer::Status) {
    println!("Component:    {}", status.component);
    println!("Resource:     {}", status.resource);
    println!("Signer:       {}", status.signer);
    println!("Fee balance:  {}", status.fee_balance);
    println!("Paused:       {}", status.paused);
    println!(
        "Total supply: {}",
        status.total_supply.as_deref().unwrap_or("not tracked")
    );
    println!(
        "Max supply:   {}",
        status.max_supply.as_deref().unwrap_or("uncapped")
    );
    println!("Treasury:     {}", status.treasury_balance);
    println!("Roles:");
    for role in &status.roles {
        let mine = if role.held_by_signer {
            " (this key)"
        } else {
            ""
        };
        println!("  {:<11} {}{mine}", role.role, role.rule);
    }
    if let Some(pending) = &status.pending_governor {
        let state = if pending.confirmed {
            "confirmed"
        } else {
            "awaiting confirmation"
        };
        println!("Pending governor: {} ({state})", pending.rule);
    }
}
