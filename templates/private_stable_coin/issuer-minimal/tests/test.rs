use ootle_byte_type::ToByteType;
// The template crate is a `cdylib` (so the wasm stays small — adding an `rlib` to
// `crate-type` inflates the wasm by ~47%). Integration tests can't link a cdylib, so
// pull the config module straight into the test crate instead.
#[path = "../src/config.rs"]
mod config;
use config::StableCoinConfig;
#[path = "../src/roles.rs"]
#[allow(dead_code)] // the tests only build role configs
mod roles;
use roles::{Role, RoleConfig};
use tari_template_lib::prelude::rule;
use tari_template_lib::types::constants::{PUBLIC_IDENTITY_RESOURCE_ADDRESS, TARI_TOKEN};
use tari_template_lib::types::{
    AccessRule, Amount, ComponentAddress, Metadata, NonFungibleAddress, NonFungibleId,
    ResourceAddress, UtxoId,
};
use tari_template_test_tooling::TemplateTest;
use tari_template_test_tooling::crypto::{PublicKey, RistrettoPublicKey, RistrettoSecretKey};
use tari_template_test_tooling::support::assert_error::assert_reject_reason;
use tari_template_test_tooling::transaction::{Transaction, args};

/// A transaction signed by `$key` calling `$method` on `$component`.
macro_rules! call {
    ($test:expr, $component:expr, $method:expr, $args:expr, $key:expr) => {
        $test
            .transaction()
            .call_method($component, $method, $args)
            .build_and_seal($key)
    };
}

const INITIAL_SUPPLY: u64 = 1_000_000_000_000_000u64;

#[test]
fn it_increases_and_decreases_supply() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
        ..
    } = setup();

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![123],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    assert_eq!(total_supply(&test, token_resource), INITIAL_SUPPLY + 123);

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "decrease_supply",
            args![456],
            &admin_key
        ),
        vec![admin_proof],
    );
    assert_eq!(
        total_supply(&test, token_resource),
        INITIAL_SUPPLY + 123 - 456
    );
}

#[test]
fn it_allows_anyone_to_hold_and_transfer_the_coin() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Neither party is registered with the issuer
    test.execute_expect_success(
        test.transaction()
            .call_method(alice_account, "withdraw", args![token_resource, 456])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(bob_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&alice_key),
        vec![alice_proof],
    );

    let vaults = test
        .read_only_state_store()
        .get_vaults_for_account(bob_account)
        .unwrap();
    assert_eq!(vaults.get(&token_resource).unwrap().balance(), 456);
}

#[test]
fn it_recalls_tokens_from_user() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
        ..
    } = setup();

    let (alice_account, _, _) = test.create_empty_account();

    test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "withdraw", args![500])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    let vault_id = test
        .read_only_state_store()
        .get_account(alice_account)
        .unwrap()
        .get_vault_by_resource(&token_resource)
        .unwrap()
        .vault_id();

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "recall_revealed_tokens",
            args![vault_id, 200],
            &admin_key
        ),
        vec![admin_proof],
    );

    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 300);

    // Recalled coins move to the issuer vault; they are not burnt
    assert_eq!(total_supply(&test, token_resource), INITIAL_SUPPLY);
}

#[test]
fn it_denies_privileged_methods_to_other_signers() {
    let TestSetup {
        mut test,
        stable_coin_component,
        ..
    } = setup();
    let (_, alice_proof, alice_key) = test.create_empty_account();
    let utxos: Vec<UtxoId> = Vec::new();

    for (method, args) in [
        ("increase_supply", args![100]),
        ("decrease_supply", args![100]),
        ("withdraw", args![100]),
        ("pause", args![]),
        ("unpause", args![]),
        ("freeze_utxos", args![utxos.clone()]),
        ("set_role", args![Role::Minter, signer_rule(&alice_key)]),
        ("propose_governor", args![signer_rule(&alice_key)]),
        ("confirm_governor", args![]),
        ("set_config_max_supply", args![None::<Amount>]),
    ] {
        let reason = test.execute_expect_failure(
            call!(test, stable_coin_component, method, args, &alice_key),
            vec![alice_proof.clone()],
        );
        assert_reject_reason(&reason, "Access Denied");
    }
}

#[test]
fn it_restricts_each_role_to_its_own_methods() {
    let mut keys = Vec::new();
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof: governor_proof,
        admin_key: governor_key,
        token_resource,
        ..
    } = setup_with_roles(|test, governor| {
        let (_, minter_proof, minter_key) = test.create_empty_account();
        let (_, pauser_proof, pauser_key) = test.create_empty_account();
        let roles = RoleConfig {
            minter: Some(signer_rule(&minter_key)),
            pauser: Some(signer_rule(&pauser_key)),
            ..governed_by(governor)
        };
        keys.push((minter_proof, minter_key));
        keys.push((pauser_proof, pauser_key));
        roles
    });
    let (pauser_proof, pauser_key) = keys.pop().unwrap();
    let (minter_proof, minter_key) = keys.pop().unwrap();

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &minter_key
        ),
        vec![minter_proof.clone()],
    );
    assert_eq!(total_supply(&test, token_resource), INITIAL_SUPPLY + 100);

    // The pauser cannot mint and the minter cannot pause
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &pauser_key
        ),
        vec![pauser_proof.clone()],
    );
    assert_reject_reason(&reason, "Access Denied");
    let reason = test.execute_expect_failure(
        call!(test, stable_coin_component, "pause", args![], &minter_key),
        vec![minter_proof.clone()],
    );
    assert_reject_reason(&reason, "Access Denied");

    // The pauser pauses, which stops minting
    test.execute_expect_success(
        call!(test, stable_coin_component, "pause", args![], &pauser_key),
        vec![pauser_proof.clone()],
    );
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &minter_key
        ),
        vec![minter_proof.clone()],
    );
    assert_reject_reason(&reason, "is paused");

    // Only the governor can unpause
    let reason = test.execute_expect_failure(
        call!(test, stable_coin_component, "unpause", args![], &pauser_key),
        vec![pauser_proof],
    );
    assert_reject_reason(&reason, "Access Denied");
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "unpause",
            args![],
            &governor_key
        ),
        vec![governor_proof],
    );
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &minter_key
        ),
        vec![minter_proof],
    );
}

#[test]
fn it_rejects_a_role_rule_that_is_not_signer_keys() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
        ..
    } = setup();

    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "set_role",
            args![Role::Minter, AccessRule::AllowAll],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "cannot be open to everyone");

    let foreign_rules = [
        rule!(resource(token_resource)),
        rule!(any_of(
            public_key(RistrettoPublicKey::from_secret_key(&admin_key).to_byte_type()),
            resource(token_resource)
        )),
    ];
    for foreign_rule in foreign_rules {
        for (method, args) in [
            ("set_role", args![Role::Minter, foreign_rule.clone()]),
            ("propose_governor", args![foreign_rule.clone()]),
        ] {
            let reason = test.execute_expect_failure(
                call!(test, stable_coin_component, method, args, &admin_key),
                vec![admin_proof.clone()],
            );
            assert_reject_reason(&reason, "may only name signer keys");
        }
    }
}

#[test]
fn it_rejects_instantiation_with_a_role_rule_that_is_not_signer_keys() {
    let mut test = TemplateTest::my_crate();
    let (_, admin_proof, admin_key) = test.create_funded_account();

    // The governor must be held by someone
    let reason = test.execute_expect_failure(
        instantiate_transaction(
            &test,
            &admin_key,
            StableCoinConfig::default(),
            governed_by(AccessRule::DenyAll),
        ),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "The governor role must be held by someone");

    let roles = RoleConfig {
        minter: Some(AccessRule::AllowAll),
        ..governed_by(signer_rule(&admin_key))
    };
    let reason = test.execute_expect_failure(
        instantiate_transaction(&test, &admin_key, StableCoinConfig::default(), roles),
        vec![admin_proof],
    );
    assert_reject_reason(&reason, "cannot be open to everyone");
}

/// A test transaction cannot reference the system resources (they are not in the test state), so the guard is
/// checked directly for rules naming them.
#[test]
fn it_only_accepts_signer_keys_in_a_role_rule() {
    let foreign_rules = [
        rule!(resource(PUBLIC_IDENTITY_RESOURCE_ADDRESS)),
        rule!(resource(TARI_TOKEN)),
        rule!(non_fungible(NonFungibleAddress::new(
            ResourceAddress::from_hex(&"ef".repeat(32)).unwrap(),
            NonFungibleId::from_u64(0)
        ))),
        rule!(any_caller_component),
        rule!(any_caller_template),
        rule!(caller_component(
            ComponentAddress::from_hex(&"cd".repeat(32)).unwrap()
        )),
    ];
    for foreign_rule in &foreign_rules {
        for role in [Role::Governor, Role::Minter] {
            let result =
                std::panic::catch_unwind(|| roles::assert_valid_role_rule(role, foreign_rule));
            assert!(result.is_err());
        }
    }
    let key = |n: u8| {
        RistrettoPublicKey::from_secret_key(&RistrettoSecretKey::from(u64::from(n))).to_byte_type()
    };
    let allowed = rule!(m_of_n(
        2,
        public_key(key(1)),
        public_key(key(2)),
        public_key(key(3))
    ));
    roles::assert_valid_role_rule(Role::Governor, &allowed);
}

#[test]
fn it_reassigns_a_role_away_from_a_stolen_key() {
    let mut keys = Vec::new();
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup_with_roles(|test, governor| {
        let (_, stolen_proof, stolen_key) = test.create_empty_account();
        let roles = RoleConfig {
            minter: Some(signer_rule(&stolen_key)),
            ..governed_by(governor)
        };
        keys.push((stolen_proof, stolen_key));
        roles
    });
    let (stolen_proof, stolen_key) = keys.pop().unwrap();
    let (_, minter_proof, minter_key) = test.create_empty_account();

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &stolen_key
        ),
        vec![stolen_proof.clone()],
    );

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "set_role",
            args![Role::Minter, signer_rule(&minter_key)],
            &admin_key
        ),
        vec![admin_proof],
    );

    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &stolen_key
        ),
        vec![stolen_proof],
    );
    assert_reject_reason(&reason, "Access Denied");
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &minter_key
        ),
        vec![minter_proof],
    );
}

#[test]
fn it_governs_with_a_threshold_of_keys() {
    let mut signers = Vec::new();
    let TestSetup {
        mut test,
        stable_coin_component,
        ..
    } = setup_with_roles(|test, _| {
        for _ in 0..3 {
            let (_, proof, key) = test.create_empty_account();
            signers.push((proof, key));
        }
        let pk = |i: usize| RistrettoPublicKey::from_secret_key(&signers[i].1).to_byte_type();
        governed_by(rule!(m_of_n(
            2,
            public_key(pk(0)),
            public_key(pk(1)),
            public_key(pk(2))
        )))
    });
    let (_, minter_proof, minter_key) = test.create_empty_account();
    let set_minter = |test: &TemplateTest| {
        call!(
            test,
            stable_coin_component,
            "set_role",
            args![Role::Minter, signer_rule(&minter_key)],
            &signers[0].1
        )
    };

    // One signer is below the threshold
    let reason = test.execute_expect_failure(set_minter(&test), vec![signers[0].0.clone()]);
    assert_reject_reason(&reason, "Access Denied");

    test.execute_expect_success(
        set_minter(&test),
        vec![signers[0].0.clone(), signers[2].0.clone()],
    );
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &minter_key
        ),
        vec![minter_proof],
    );
}

#[test]
fn it_rotates_the_governor() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();
    let (_, governor_proof, governor_key) = test.create_empty_account();
    let (_, other_proof, other_key) = test.create_empty_account();
    let new_governor = signer_rule(&governor_key);
    let hand_over = |test: &TemplateTest, rule: AccessRule| {
        call!(
            test,
            stable_coin_component,
            "set_role",
            args![Role::Governor, rule],
            &admin_key
        )
    };

    // No proposal
    let reason = test.execute_expect_failure(
        hand_over(&test, new_governor.clone()),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "No governor proposal is pending");

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "propose_governor",
            args![new_governor.clone()],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );

    // Proposed but not confirmed
    let reason = test.execute_expect_failure(
        hand_over(&test, new_governor.clone()),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "The proposed governor has not confirmed");

    // Only the proposed governor can confirm
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "confirm_governor",
            args![],
            &other_key
        ),
        vec![other_proof],
    );
    assert_reject_reason(&reason, "Access Denied");
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "confirm_governor",
            args![],
            &governor_key
        ),
        vec![governor_proof.clone()],
    );

    // The handover must name the confirmed rule
    let reason = test.execute_expect_failure(
        hand_over(&test, signer_rule(&other_key)),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "does not match the pending governor proposal");

    test.execute_expect_success(hand_over(&test, new_governor), vec![admin_proof.clone()]);

    // The old governor no longer governs
    let reason = test.execute_expect_failure(
        call!(test, stable_coin_component, "unpause", args![], &admin_key),
        vec![admin_proof],
    );
    assert_reject_reason(&reason, "Access Denied");

    // The new governor reassigns roles, and the proposal is spent
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "set_role",
                args![Role::Minter, signer_rule(&governor_key)],
            )
            .call_method(stable_coin_component, "increase_supply", args![100])
            .build_and_seal(&governor_key),
        vec![governor_proof.clone()],
    );
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "confirm_governor",
            args![],
            &governor_key
        ),
        vec![governor_proof],
    );
    assert_reject_reason(&reason, "No governor proposal is pending");
}

#[test]
fn it_cancels_a_governor_proposal() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();
    let (_, governor_proof, governor_key) = test.create_empty_account();

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "propose_governor",
            args![signer_rule(&governor_key)],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "cancel_governor_proposal",
            args![],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );

    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "confirm_governor",
            args![],
            &governor_key
        ),
        vec![governor_proof],
    );
    assert_reject_reason(&reason, "Access Denied");
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "cancel_governor_proposal",
            args![],
            &admin_key
        ),
        vec![admin_proof],
    );
    assert_reject_reason(&reason, "No governor proposal is pending");
}

#[test]
fn it_caps_supply_at_max_supply() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
        ..
    } = setup();
    let max_supply = Amount::from(INITIAL_SUPPLY + 100);
    let set_max = |test: &TemplateTest, max: Option<Amount>| {
        call!(
            test,
            stable_coin_component,
            "set_config_max_supply",
            args![max],
            &admin_key
        )
    };
    let mint = |test: &TemplateTest, amount: u64| {
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![amount],
            &admin_key
        )
    };

    test.execute_expect_success(set_max(&test, Some(max_supply)), vec![admin_proof.clone()]);

    // Up to the cap
    test.execute_expect_success(mint(&test, 100), vec![admin_proof.clone()]);
    assert_eq!(total_supply(&test, token_resource), max_supply);

    // Over the cap
    let reason = test.execute_expect_failure(mint(&test, 1), vec![admin_proof.clone()]);
    assert_reject_reason(&reason, "Minting would exceed the max supply");

    // The cap cannot drop below the current supply
    let reason = test.execute_expect_failure(
        set_max(&test, Some(max_supply - Amount::ONE)),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "The max supply is below the current supply");

    // Removing the cap restores unlimited minting
    test.execute_expect_success(set_max(&test, None), vec![admin_proof.clone()]);
    test.execute_expect_success(mint(&test, 1), vec![admin_proof]);
}

#[test]
fn it_caps_freeze_batches() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();
    let utxos: Vec<UtxoId> = (0..=100u8).map(|i| UtxoId::from_array([i; 32])).collect();

    for method in ["freeze_utxos", "unfreeze_utxos"] {
        let reason = test.execute_expect_failure(
            call!(
                test,
                stable_coin_component,
                method,
                args![utxos.clone()],
                &admin_key
            ),
            vec![admin_proof.clone()],
        );
        assert_reject_reason(&reason, "Too many UTXOs in one batch");
    }
}

#[test]
fn it_rejects_an_initial_supply_above_max_supply() {
    let mut test = TemplateTest::my_crate();
    let (_, admin_proof, admin_key) = test.create_funded_account();
    let config = StableCoinConfig {
        max_supply: Some(Amount::from(INITIAL_SUPPLY - 1)),
    };

    let reason = test.execute_expect_failure(
        instantiate_transaction(
            &test,
            &admin_key,
            config,
            governed_by(signer_rule(&admin_key)),
        ),
        vec![admin_proof],
    );
    assert_reject_reason(&reason, "The initial supply exceeds the max supply");
}

struct TestSetup {
    test: TemplateTest,
    stable_coin_component: ComponentAddress,
    /// The signer badge and key of the governor `setup` instantiates with.
    admin_proof: NonFungibleAddress,
    admin_key: RistrettoSecretKey,
    token_resource: ResourceAddress,
}

fn signer_rule(key: &RistrettoSecretKey) -> AccessRule {
    rule!(public_key(
        RistrettoPublicKey::from_secret_key(key).to_byte_type()
    ))
}

/// A role config where `governor` holds every role.
fn governed_by(governor: AccessRule) -> RoleConfig {
    RoleConfig {
        governor,
        minter: None,
        burner: None,
        treasurer: None,
        compliance: None,
        pauser: None,
    }
}

fn total_supply(test: &TemplateTest, token_resource: ResourceAddress) -> Amount {
    test.read_only_state_store()
        .get_resource(&token_resource)
        .unwrap()
        .total_supply()
        .unwrap()
}

fn instantiate_transaction(
    test: &TemplateTest,
    signer_key: &RistrettoSecretKey,
    config: StableCoinConfig,
    roles: RoleConfig,
) -> Transaction {
    let template = test.get_template_address("TariStableCoin");
    let mut metadata = Metadata::new();
    metadata
        .insert("provider_name", "Stable coinz 4 U")
        .insert("collateralized_by", "Z$")
        .insert("issuing_authority", "Bank of Silly Walks")
        .insert("issued_at", "2023-01-01");

    let view_key = RistrettoPublicKey::from_secret_key(signer_key).to_byte_type();
    test.transaction()
        .allocate_component_address("stable_coin_addr")
        .call_function(
            template,
            "instantiate",
            args![
                Workspace("stable_coin_addr"),
                INITIAL_SUPPLY,
                "SC4U",
                metadata,
                8,
                view_key,
                config,
                roles
            ],
        )
        .build_and_seal(signer_key)
}

/// Instantiates the stable coin with a single signer key holding every role.
fn setup() -> TestSetup {
    setup_with_roles(|_, governor| governed_by(governor))
}

/// Instantiates the stable coin with the roles `roles` returns. `roles` is given a rule naming the admin key, and may
/// create accounts in the test first.
fn setup_with_roles(roles: impl FnOnce(&mut TemplateTest, AccessRule) -> RoleConfig) -> TestSetup {
    let mut test = TemplateTest::my_crate();
    let (_, admin_proof, admin_key) = test.create_funded_account();
    let roles = roles(&mut test, signer_rule(&admin_key));
    let template = test.get_template_address("TariStableCoin");
    let result = test.execute_expect_success(
        instantiate_transaction(&test, &admin_key, StableCoinConfig::default(), roles),
        vec![admin_proof.clone()],
    );

    let stable_coin_component = result
        .finalize
        .result
        .any_accept()
        .unwrap()
        .up_iter()
        .find(|(id, s)| {
            id.is_component()
                && *s.substate_value().component().unwrap().template_address() == template
        })
        .map(|(id, _)| id.as_component_address().unwrap())
        .unwrap();

    // Component state is encoded with minicbor as a positional array, so look up fields by
    // their #[n(N)] index rather than by name.
    let token_vault = test
        .read_only_state_store()
        .inspect_component(stable_coin_component)
        .unwrap()
        .get_value("$.1")
        .unwrap()
        .expect("token_vault not found");
    let token_resource = *test
        .read_only_state_store()
        .get_vault(&token_vault)
        .unwrap()
        .resource_address();

    TestSetup {
        test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
    }
}
