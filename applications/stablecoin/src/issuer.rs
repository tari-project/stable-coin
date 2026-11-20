//! The issuer client: builds, signs and submits stable coin transactions through an indexer, and
//! reads the coin's state back.

use std::{fs, path::Path};

use anyhow::{Context, anyhow, bail};
use ootle_byte_type::ToByteType;
use ootle_rs::{
    Address, Epoch, ToAccountAddress, TransactionOutcome, TransactionRequest,
    builtin_templates::{
        UnsignedTransactionBuilder,
        account::IAccount,
        component::{IComponent, TransactionBuildable},
        faucet::IFaucet,
    },
    key_provider::PrivateKeyProvider,
    keys::OotleSecretKey,
    provider::{IndexerProvider, ProviderBuilder, WalletProvider},
    template_types::{
        Amount, ComponentAddress, ResourceAddress, TemplateAddress, UtxoId, VaultId,
        constants::TARI_TOKEN, crypto::RistrettoPublicKeyBytes,
    },
    transaction::TransactionSigner,
    wallet::OotleWallet,
};
use serde::{Deserialize, Serialize};
use tari_crypto::{keys::PublicKey, ristretto::RistrettoPublicKey};
use tari_ootle_common_types::engine_types::{
    component::Component, events::Event, indexed_value::IndexedWellKnownTypes, substate::SubstateId,
};
use tari_ootle_transaction::{UnsignedTransaction, args, builder::named_args::NamedArg};
use tari_template_lib::{
    prelude::{AccessRule, AccessRules, Metadata, OwnerRule, rule},
    types::{SubstateOwnerRule, UtxoAddress, access_rules::RuleRequirement},
};

use crate::{
    config::StableCoinConfig,
    roles::{Role, RoleConfig},
};

/// The fee ceiling attached to every transaction. Unused fees are not charged.
const MAX_FEE: u64 = 20_000;
/// Publishing a template is charged by binary size, so it gets its own ceiling.
const MAX_PUBLISH_FEE: u64 = 5_000_000;
/// How many epochs a transaction stays valid for.
const EPOCH_WINDOW: u64 = 10;

/// Where a coin was deployed. Written by `deploy` and read by every other command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub indexer_url: String,
    pub template: TemplateAddress,
    pub component: ComponentAddress,
    pub resource: ResourceAddress,
}

impl Deployment {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("No deployment at {}: run `deploy` first", path.display()))?;
        serde_json::from_str(&contents).context("Invalid deployment file")
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        fs::write(path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("Failed to write {}", path.display()))
    }
}

pub struct DeployParams {
    pub template: TemplateAddress,
    pub initial_supply: Amount,
    pub symbol: String,
    pub name: String,
    pub divisibility: u8,
    pub max_supply: Option<Amount>,
    pub roles: RoleConfig,
}

/// A privileged operation on the coin, shared by the command line and the web dashboard.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Mint into the issuer vault (minter).
    Mint {
        amount: u64,
    },
    /// Burn from the issuer vault (burner).
    Burn {
        amount: u64,
    },
    /// Send from the issuer vault to an address (treasurer).
    Send {
        to: String,
        amount: u64,
    },
    /// Pull a revealed amount out of an account into the issuer vault (compliance).
    Recall {
        from: String,
        amount: u64,
    },
    Freeze {
        utxos: Vec<String>,
    },
    Unfreeze {
        utxos: Vec<String>,
    },
    Pause,
    Unpause,
    /// Give a role to a key, or back to the governor with the holder `governor` (governor).
    SetRole {
        role: String,
        holder: String,
    },
    /// Hand governance to another key (governor).
    SetGovernor {
        holder: String,
    },
    SetMaxSupply {
        max_supply: Option<u64>,
    },
}

/// What a committed transaction did.
#[derive(Debug, Clone, Serialize)]
pub struct TxSummary {
    pub transaction_id: String,
    pub fee: String,
    pub events: Vec<String>,
    #[serde(skip)]
    pub upped: Vec<SubstateId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub component: String,
    pub resource: String,
    pub signer: String,
    pub signer_public_key: String,
    pub fee_balance: String,
    pub paused: bool,
    pub total_supply: Option<String>,
    pub max_supply: Option<String>,
    pub treasury_balance: String,
    pub roles: Vec<RoleStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleStatus {
    pub role: String,
    pub rule: String,
    /// Whether the connected key alone satisfies this role's rule.
    pub held_by_signer: bool,
}

/// The component state of the `issuer-minimal` template, field for field.
#[derive(minicbor::Decode)]
struct CoinState {
    #[n(0)]
    config: StableCoinConfig,
    #[n(1)]
    token_vault: VaultId,
    #[n(2)]
    is_paused: bool,
}

pub struct Issuer {
    provider: IndexerProvider<OotleWallet>,
    address: Address,
    public_key: RistrettoPublicKeyBytes,
    view_key: RistrettoPublicKeyBytes,
}

impl Issuer {
    pub async fn connect(key: OotleSecretKey, indexer_url: &str) -> anyhow::Result<Self> {
        let view_key = RistrettoPublicKey::from_secret_key(key.view_only_secret()).to_byte_type();
        let signer = PrivateKeyProvider::new(key);
        let address = signer.address().clone();
        let public_key = *address.account_public_key();
        let provider = ProviderBuilder::new()
            .wallet(OotleWallet::from(signer))
            .connect(indexer_url)
            .await
            .with_context(|| format!("Failed to connect to the indexer at {indexer_url}"))?;
        Ok(Self {
            provider,
            address,
            public_key,
            view_key,
        })
    }

    /// A rule satisfied by the connected key.
    pub fn signer_rule(&self) -> AccessRule {
        key_rule(self.public_key)
    }

    /// Takes test funds from the network faucet. Only test networks have one.
    pub async fn faucet(&mut self) -> anyhow::Result<TxSummary> {
        let unsigned = IFaucet::new(&self.provider, self.max_epoch().await?)
            .take_faucet_funds()
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Publishes a template binary and returns its address.
    pub async fn publish_template(
        &mut self,
        wasm: Vec<u8>,
    ) -> anyhow::Result<(TemplateAddress, TxSummary)> {
        self.ensure_funded().await?;
        let unsigned = IAccount::new(&self.provider, self.max_epoch().await?)
            .publish_template(wasm)
            .pay_fee(MAX_PUBLISH_FEE)
            .prepare()
            .await?;
        let summary = self.submit(unsigned).await?;
        let template = summary
            .upped
            .iter()
            .find_map(|id| match id {
                SubstateId::Template(address) => Some(address.as_template_address()),
                _ => None,
            })
            .ok_or_else(|| anyhow!("The transaction did not publish a template"))?;
        Ok((template, summary))
    }

    /// Instantiates the coin. The connected key's view key becomes the resource view key.
    pub async fn deploy(
        &mut self,
        params: DeployParams,
    ) -> anyhow::Result<(Deployment, TxSummary)> {
        self.ensure_funded().await?;
        let mut metadata = Metadata::new();
        metadata.insert("provider_name", params.name.as_str());
        let config = StableCoinConfig {
            max_supply: params.max_supply,
        };
        let view_key = self.view_key;
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .then(|builder| {
                builder.allocate_component_address("coin").call_function(
                    params.template,
                    "instantiate",
                    args![
                        Workspace("coin"),
                        params.initial_supply,
                        params.symbol,
                        metadata,
                        params.divisibility,
                        view_key,
                        Some(config),
                        params.roles
                    ],
                )
            })
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        let summary = self.submit(unsigned).await?;

        let component = summary
            .upped
            .iter()
            .filter_map(|id| id.as_component_address())
            .find(|address| *address != self.address.to_account_address())
            .ok_or_else(|| anyhow!("The transaction did not create a component"))?;
        let resource = summary
            .upped
            .iter()
            .filter_map(|id| id.as_resource_address())
            .find(|address| *address != TARI_TOKEN)
            .ok_or_else(|| anyhow!("The transaction did not create a resource"))?;
        let deployment = Deployment {
            indexer_url: self.provider.indexer_url().to_string(),
            template: params.template,
            component,
            resource,
        };
        Ok((deployment, summary))
    }

    /// Calls a method on the coin component.
    pub async fn call(
        &mut self,
        deployment: &Deployment,
        method: &str,
        args: Vec<NamedArg>,
    ) -> anyhow::Result<TxSummary> {
        self.ensure_funded().await?;
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .call_method(deployment.component, method, args)
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Gives `role` to whoever satisfies `rule`, or back to the governor with `None`.
    ///
    /// The component's access rules are replaced as a whole, so the current rules are read first and every other
    /// method keeps its rule.
    pub async fn set_role(
        &mut self,
        deployment: &Deployment,
        role: Role,
        rule: Option<AccessRule>,
    ) -> anyhow::Result<TxSummary> {
        self.ensure_funded().await?;
        let current = self.component(deployment).await?;
        let rules = with_role(current.access_rules(), role, rule);
        let component = deployment.component;
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .want_substate(component.into(), true)
            .then(|builder| builder.set_component_access_rules(component, rules))
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Hands the component, and with it every role left with the governor, to whoever satisfies `rule`. Nothing
    /// checks that anyone does: a rule no one holds leaves the coin without a governor for good.
    pub async fn set_governor(
        &mut self,
        deployment: &Deployment,
        rule: AccessRule,
    ) -> anyhow::Result<TxSummary> {
        self.ensure_funded().await?;
        let component = deployment.component;
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .want_substate(component.into(), true)
            .then(|builder| {
                builder.set_component_owner_rule(component, OwnerRule::ByAccessRule(rule))
            })
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Withdraws from the issuer vault and deposits into `recipient`'s account, creating it if needed.
    pub async fn withdraw_to(
        &mut self,
        deployment: &Deployment,
        recipient: &Address,
        amount: Amount,
    ) -> anyhow::Result<TxSummary> {
        self.ensure_funded().await?;
        let recipient_key = *recipient.account_public_key();
        let recipient_account = recipient.to_account_address();
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .call_method(deployment.component, "withdraw", args![amount])
            // An account that already exists, and its vault for the coin, must be inputs: without them
            // the transaction tries to create the account again.
            .want_substate(recipient_account.into(), false)
            .want_vault_for(recipient_account, deployment.resource, false)
            .then(|builder| {
                builder
                    .put_last_instruction_output_on_workspace("coins")
                    .create_account_with_bucket(recipient_key, "coins")
            })
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Recalls a revealed amount of the coin from `holder`'s account into the issuer vault.
    pub async fn recall_from(
        &mut self,
        deployment: &Deployment,
        holder: &Address,
        amount: Amount,
    ) -> anyhow::Result<TxSummary> {
        self.ensure_funded().await?;
        let vault = self.account_vault(holder, deployment.resource).await?;
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .call_method(
                deployment.component,
                "recall_revealed_tokens",
                args![vault, amount],
            )
            .then(|builder| builder.add_input(SubstateId::Vault(vault)))
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Calls `freeze_utxos` or `unfreeze_utxos`. Each UTXO is a substate the transaction changes, so each must be
    /// an input.
    async fn freeze(
        &mut self,
        deployment: &Deployment,
        method: &str,
        utxos: Vec<UtxoId>,
    ) -> anyhow::Result<TxSummary> {
        self.ensure_funded().await?;
        let inputs: Vec<_> = utxos
            .iter()
            .map(|id| SubstateId::Utxo(UtxoAddress::new(deployment.resource, *id)))
            .collect();
        let unsigned = IComponent::new(&self.provider, self.max_epoch().await?)
            .call_method(deployment.component, method, args![utxos])
            .then(|builder| {
                inputs
                    .into_iter()
                    .fold(builder, |b, input| b.add_input(input))
            })
            .pay_fee(MAX_FEE)
            .prepare()
            .await?;
        self.submit(unsigned).await
    }

    /// Runs `action` and returns a one-line description with the transaction summary.
    pub async fn execute(
        &mut self,
        deployment: &Deployment,
        action: Action,
    ) -> anyhow::Result<(String, TxSummary)> {
        let (label, method, args) = match action {
            Action::Send { to, amount } => {
                let recipient = parse_address(&to)?;
                let summary = self
                    .withdraw_to(deployment, &recipient, Amount::from(amount))
                    .await?;
                return Ok((format!("Sent {amount} to {to}"), summary));
            }
            Action::Recall { from, amount } => {
                let holder = parse_address(&from)?;
                let summary = self
                    .recall_from(deployment, &holder, Amount::from(amount))
                    .await?;
                return Ok((format!("Recalled {amount} from {from}"), summary));
            }
            Action::Mint { amount } => (
                format!("Minted {amount}"),
                "increase_supply",
                args![Amount::from(amount)],
            ),
            Action::Burn { amount } => (
                format!("Burnt {amount}"),
                "decrease_supply",
                args![Amount::from(amount)],
            ),
            Action::Freeze { utxos } => {
                let summary = self
                    .freeze(deployment, "freeze_utxos", parse_utxos(&utxos)?)
                    .await?;
                return Ok((format!("Froze {} UTXO(s)", utxos.len()), summary));
            }
            Action::Unfreeze { utxos } => {
                let summary = self
                    .freeze(deployment, "unfreeze_utxos", parse_utxos(&utxos)?)
                    .await?;
                return Ok((format!("Unfroze {} UTXO(s)", utxos.len()), summary));
            }
            Action::Pause => ("Paused".to_string(), "pause", args![]),
            Action::Unpause => ("Unpaused".to_string(), "unpause", args![]),
            Action::SetRole { role, holder } => {
                let role = parse_role(&role)?;
                let rule = if holder.eq_ignore_ascii_case("governor") {
                    None
                } else {
                    Some(parse_holder(&holder)?)
                };
                let summary = self.set_role(deployment, role, rule).await?;
                return Ok((format!("Set {role:?} to {holder}"), summary));
            }
            Action::SetGovernor { holder } => {
                let summary = self
                    .set_governor(deployment, parse_holder(&holder)?)
                    .await?;
                return Ok((format!("Handed governance to {holder}"), summary));
            }
            Action::SetMaxSupply { max_supply } => (
                match max_supply {
                    Some(max) => format!("Set the max supply to {max}"),
                    None => "Removed the max supply".to_string(),
                },
                "set_config_max_supply",
                args![max_supply.map(Amount::from)],
            ),
        };
        let summary = self.call(deployment, method, args).await?;
        Ok((label, summary))
    }

    pub async fn status(&self, deployment: &Deployment) -> anyhow::Result<Status> {
        let component = self.component(deployment).await?;
        let state: CoinState = tari_bor::from_value(component.state())
            .context("Unexpected component state: wrong template?")?;

        let resource = self.provider.get_resource(deployment.resource).await?;
        let treasury = self
            .provider
            .get_substate(state.token_vault)
            .await?
            .and_then(|s| s.into_substate_value().into_vault())
            .map(|vault| vault.balance())
            .unwrap_or_default();
        let fee_balance = self
            .provider
            .get_account_balance(self.address.to_account_address(), TARI_TOKEN)
            .await
            .map(|balance| balance.to_string())
            .unwrap_or_else(|_| "0".to_string());

        let governor = match component.owner_rule() {
            SubstateOwnerRule::ByAccessRule(rule) => RoleStatus {
                role: "Governor".to_string(),
                rule: describe_rule(rule),
                held_by_signer: *rule == self.signer_rule(),
            },
            SubstateOwnerRule::ByPublicKey(key) => RoleStatus {
                role: "Governor".to_string(),
                rule: key.to_string(),
                held_by_signer: *key == self.public_key,
            },
            SubstateOwnerRule::None => RoleStatus {
                role: "Governor".to_string(),
                rule: "no one".to_string(),
                held_by_signer: false,
            },
        };
        let mut roles = vec![governor.clone()];
        roles.extend(Role::ALL.iter().map(|role| {
            // A role's methods share one rule; a role without one is left with the governor.
            match role_rule(component.access_rules(), *role) {
                Some(rule) => RoleStatus {
                    role: format!("{role:?}"),
                    rule: describe_rule(rule),
                    held_by_signer: *rule == self.signer_rule(),
                },
                None => RoleStatus {
                    role: format!("{role:?}"),
                    rule: "the governor".to_string(),
                    held_by_signer: governor.held_by_signer,
                },
            }
        }));

        Ok(Status {
            component: deployment.component.to_string(),
            resource: deployment.resource.to_string(),
            signer: self.address.to_string(),
            signer_public_key: self.public_key.to_string(),
            fee_balance,
            paused: state.is_paused,
            total_supply: resource.total_supply().map(|a| a.to_string()),
            max_supply: state.config.max_supply.map(|a| a.to_string()),
            treasury_balance: treasury.to_string(),
            roles,
        })
    }

    async fn component(&self, deployment: &Deployment) -> anyhow::Result<Component> {
        self.provider
            .get_substate(deployment.component)
            .await?
            .ok_or_else(|| anyhow!("Component {} not found", deployment.component))?
            .into_substate_value()
            .into_component()
            .ok_or_else(|| anyhow!("{} is not a component", deployment.component))
    }

    /// The vault holding `resource` in `holder`'s account.
    async fn account_vault(
        &self,
        holder: &Address,
        resource: ResourceAddress,
    ) -> anyhow::Result<VaultId> {
        let account = holder.to_account_address();
        let component = self
            .provider
            .get_substate(account)
            .await?
            .and_then(|s| s.into_substate_value().into_component())
            .ok_or_else(|| anyhow!("{holder} has no account"))?;
        let indexed =
            IndexedWellKnownTypes::from_value(component.state()).map_err(|e| anyhow!("{e}"))?;
        for vault_id in indexed.vault_ids() {
            let vault = self
                .provider
                .get_substate(*vault_id)
                .await?
                .and_then(|s| s.into_substate_value().into_vault());
            if vault.is_some_and(|vault| *vault.resource_address() == resource) {
                return Ok(*vault_id);
            }
        }
        bail!("{holder} holds no revealed balance of this coin")
    }

    async fn max_epoch(&self) -> anyhow::Result<Epoch> {
        Ok(Epoch(
            self.provider.get_epoch().await?.as_u64() + EPOCH_WINDOW,
        ))
    }

    /// Fees are paid from the signing key's account, which exists only once the address has been funded.
    async fn ensure_funded(&self) -> anyhow::Result<()> {
        let account = self.address.to_account_address();
        if self.provider.get_substate(account).await?.is_none() {
            bail!(
                "This key has no account on the network yet, so it cannot pay fees. Send public TARI to {} or run \
                 `faucet` on a test network.",
                self.address
            );
        }
        Ok(())
    }

    async fn submit(&mut self, mut unsigned: UnsignedTransaction) -> anyhow::Result<TxSummary> {
        // Without a nonce, repeating a command builds the same transaction, and the network answers with the
        // result of the first one instead of running it again.
        unsigned.set_nonce(rand::random());
        let transaction = TransactionRequest::default()
            .with_transaction(unsigned)
            .build(self.provider.wallet())
            .await?;
        let pending = self.provider.send_transaction(transaction).await?;
        let transaction_id = pending.tx_id().to_string();
        match pending.watch().await? {
            TransactionOutcome::Commit => {}
            TransactionOutcome::OnlyFeeCommit(reason) | TransactionOutcome::Reject(reason) => {
                bail!("Transaction {transaction_id} was rejected: {reason}")
            }
        }
        let receipt = pending.get_receipt().await?;
        Ok(TxSummary {
            transaction_id,
            fee: receipt.fee_receipt.total_fees_paid().to_string(),
            events: receipt.events.iter().filter_map(describe_event).collect(),
            upped: receipt
                .diff_summary
                .upped
                .iter()
                .map(|up| up.substate_id.clone())
                .collect(),
        })
    }
}

/// Template events only: the engine's own `std.*` events repeat what the template reports.
fn describe_event(event: &Event) -> Option<String> {
    let topic = event.topic();
    if topic.starts_with("std.") {
        return None;
    }
    let name = topic.rsplit('.').next().unwrap_or(topic);
    let payload = plain_integers(&event.payload().to_string());
    // Rules print as long nested CBOR; the status view shows them readably.
    Some(if payload.is_empty() || payload.len() > 80 {
        name.to_string()
    } else {
        format!("{name} ({payload})")
    })
}

/// Rewrites the CBOR debug form `Integer(25)` as `25`.
fn plain_integers(payload: &str) -> String {
    let mut out = String::with_capacity(payload.len());
    let mut rest = payload;
    while let Some(start) = rest.find("Integer(") {
        let digits = &rest[start + "Integer(".len()..];
        let Some(end) = digits.find(')') else { break };
        out.push_str(&rest[..start]);
        out.push_str(&digits[..end]);
        rest = &digits[end + 1..];
    }
    out.push_str(rest);
    out
}

fn parse_address(address: &str) -> anyhow::Result<Address> {
    address
        .parse()
        .map_err(|_| anyhow!("`{address}` is not an Ootle address"))
}

/// A rule satisfied by the signer key `public_key`.
pub fn key_rule(public_key: RistrettoPublicKeyBytes) -> AccessRule {
    rule!(public_key(public_key))
}

/// Parses a role holder: an Ootle address or a hex public key.
pub fn parse_holder(holder: &str) -> anyhow::Result<AccessRule> {
    if let Ok(address) = holder.parse::<Address>() {
        return Ok(key_rule(*address.account_public_key()));
    }
    let public_key = RistrettoPublicKeyBytes::from_hex(holder)
        .map_err(|_| anyhow!("`{holder}` is neither an Ootle address nor a hex public key"))?;
    Ok(key_rule(public_key))
}

pub fn parse_role(role: &str) -> anyhow::Result<Role> {
    Role::ALL
        .into_iter()
        .find(|candidate| format!("{candidate:?}").eq_ignore_ascii_case(role))
        .ok_or_else(|| {
            anyhow!(
                "Unknown role `{role}`: expected minter, burner, treasurer, compliance or pauser"
            )
        })
}

/// The rule on `role`'s methods, or `None` when the role is left with the governor.
fn role_rule(rules: &AccessRules, role: Role) -> Option<&AccessRule> {
    rules
        .method_access_rules_iter()
        .find(|(method, _)| role.methods().contains(&method.as_str()))
        .map(|(_, rule)| rule)
}

/// `rules` with `role`'s methods given `rule`, or left to the governor with `None`. Every other rule is kept.
fn with_role(rules: &AccessRules, role: Role, rule: Option<AccessRule>) -> AccessRules {
    let kept = rules
        .method_access_rules_iter()
        .filter(|(method, _)| !role.methods().contains(&method.as_str()))
        .fold(AccessRules::new(), |kept, (method, rule)| {
            kept.add_method_rule(method.clone(), rule.clone())
        })
        // No method has an empty name, so this reads the default rule.
        .default(rules.get_method_access_rule("").clone());
    let Some(rule) = rule else { return kept };
    role.methods().iter().fold(kept, |rules, method| {
        rules.add_method_rule(*method, rule.clone())
    })
}

pub fn parse_utxos(utxos: &[String]) -> anyhow::Result<Vec<UtxoId>> {
    utxos
        .iter()
        .map(|utxo| {
            let bytes: [u8; 32] = hex::decode(utxo)
                .ok()
                .and_then(|bytes| bytes.try_into().ok())
                .ok_or_else(|| anyhow!("Invalid UTXO id `{utxo}`: expected 32 bytes of hex"))?;
            Ok(UtxoId::from_array(bytes))
        })
        .collect()
}

/// Shows a single-key rule as that key, and anything else in its debug form.
fn describe_rule(rule: &AccessRule) -> String {
    let key = std::cell::Cell::new(None);
    rule.contains_requirement(&|requirement| {
        if let RuleRequirement::NonFungibleAddress(badge) = requirement {
            key.set(badge.to_public_key());
        }
        false
    });
    match key.get() {
        Some(key) if *rule == key_rule(key) => key.to_string(),
        _ => format!("{rule:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_prints_event_integers_plainly() {
        assert_eq!(plain_integers("amount = Integer(25)"), "amount = 25");
        assert_eq!(
            plain_integers("new = Null, prev = Integer(2000000000)"),
            "new = Null, prev = 2000000000"
        );
        assert_eq!(plain_integers(""), "");
    }

    #[test]
    fn it_parses_role_names_and_holders() {
        assert_eq!(parse_role("Minter").unwrap(), Role::Minter);
        assert!(parse_role("wizard").is_err());
        assert!(parse_role("governor").is_err());
        let key = "be2f2b71f42cbedd2e0a020a0d19304454174e49aa15189baae7b58f13031305";
        let rule = parse_holder(key).unwrap();
        assert_eq!(describe_rule(&rule), key);
        assert!(parse_holder("not-a-key").is_err());
        assert!(parse_utxos(&["zz".to_string()]).is_err());
        assert_eq!(parse_utxos(&["00".repeat(32)]).unwrap().len(), 1);
    }

    #[test]
    fn it_changes_one_role_and_keeps_the_others() {
        let key = |n: u8| key_rule(RistrettoPublicKeyBytes::from([n; 32]));
        let config = RoleConfig {
            governor: key(1),
            minter: Some(key(2)),
            burner: None,
            treasurer: None,
            compliance: Some(key(3)),
            pauser: None,
        };
        let rules = config.access_rules();
        assert_eq!(role_rule(&rules, Role::Minter), Some(&key(2)));
        assert_eq!(role_rule(&rules, Role::Burner), None);

        let rules = with_role(&rules, Role::Burner, Some(key(4)));
        let rules = with_role(&rules, Role::Minter, None);
        assert_eq!(role_rule(&rules, Role::Minter), None);
        assert_eq!(role_rule(&rules, Role::Compliance), Some(&key(3)));
        for method in Role::Burner.methods() {
            assert_eq!(rules.get_method_access_rule(method), &key(4));
        }
        // Unlisted methods stay closed to all but the governor
        assert_eq!(
            rules.get_method_access_rule("unpause"),
            &AccessRule::DenyAll
        );
    }
}
