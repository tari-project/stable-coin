use ootle_byte_type::ToByteType;
// The template crate is a `cdylib` (so the wasm stays small — adding an `rlib` to
// `crate-type` inflates the wasm by ~47%). Integration tests can't link a cdylib, so
// pull the config module straight into the test crate instead.
#[path = "../src/config.rs"]
mod config;
use config::StableCoinConfig;
#[path = "../src/roles.rs"]
mod roles;
use roles::{RoleConfig, Roles};
use tari_template_lib::prelude::rule;
use tari_template_lib::types::{
    AccessRule, Amount, ComponentAddress, Metadata, NonFungibleAddress, OwnerRule, ResourceAddress,
    UtxoAddress, UtxoId,
};
use tari_template_test_tooling::TemplateTest;
use tari_template_test_tooling::crypto::{PublicKey, RistrettoPublicKey, RistrettoSecretKey};
use tari_template_test_tooling::engine_types::crypto::ElgamalVerifiableBalance;
use tari_template_test_tooling::support::assert_error::assert_reject_reason;
use tari_template_test_tooling::support::stealth::{self, NO_INPUTS};
use tari_template_test_tooling::transaction::{Transaction, args};
use tari_template_test_tooling::wallet_crypto::viewable_balance_proof::generate_elgamal_value_proof;

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
        let (_, pauser_proof, pauser_key) = test.create_empty_account();
        let roles = RoleConfig {
            minter: Some(signer_rule(&stolen_key)),
            pauser: Some(signer_rule(&pauser_key)),
            ..governed_by(governor)
        };
        keys.push((pauser_proof, pauser_key));
        keys.push((stolen_proof, stolen_key));
        roles
    });
    let (stolen_proof, stolen_key) = keys.pop().unwrap();
    let (pauser_proof, pauser_key) = keys.pop().unwrap();
    let (_, minter_proof, minter_key) = test.create_empty_account();
    let mint = |test: &TemplateTest, key: &RistrettoSecretKey| {
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            key
        )
    };
    let new_roles = RoleConfig {
        minter: Some(signer_rule(&minter_key)),
        pauser: Some(signer_rule(&pauser_key)),
        ..governed_by(signer_rule(&admin_key))
    };

    test.execute_expect_success(mint(&test, &stolen_key), vec![stolen_proof.clone()]);

    // A role holder cannot replace the rules
    let reason = test.execute_expect_failure(
        test.transaction()
            .set_component_access_rules(stable_coin_component, new_roles.access_rules())
            .build_and_seal(&stolen_key),
        vec![stolen_proof.clone()],
    );
    assert_reject_reason(&reason, "Access Denied");

    test.execute_expect_success(
        test.transaction()
            .set_component_access_rules(stable_coin_component, new_roles.access_rules())
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    let reason = test.execute_expect_failure(mint(&test, &stolen_key), vec![stolen_proof]);
    assert_reject_reason(&reason, "Access Denied");
    test.execute_expect_success(mint(&test, &minter_key), vec![minter_proof]);
    // The roles that were not reassigned are untouched
    test.execute_expect_success(
        call!(test, stable_coin_component, "pause", args![], &pauser_key),
        vec![pauser_proof],
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
    let roles = RoleConfig {
        minter: Some(signer_rule(&minter_key)),
        ..governed_by(AccessRule::DenyAll)
    };
    let set_minter = |test: &TemplateTest| {
        test.transaction()
            .set_component_access_rules(stable_coin_component, roles.access_rules())
            .build_and_seal(&signers[0].1)
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
fn it_hands_over_the_governor() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();
    let (_, governor_proof, governor_key) = test.create_empty_account();
    let hand_over = |test: &TemplateTest, key: &RistrettoSecretKey| {
        test.transaction()
            .set_component_owner_rule(
                stable_coin_component,
                OwnerRule::ByAccessRule(signer_rule(&governor_key)),
            )
            .build_and_seal(key)
    };

    // Only the governor hands over
    let reason = test.execute_expect_failure(
        hand_over(&test, &governor_key),
        vec![governor_proof.clone()],
    );
    assert_reject_reason(&reason, "Access Denied");
    test.execute_expect_success(hand_over(&test, &admin_key), vec![admin_proof.clone()]);

    // The old governor no longer governs, nor holds the roles that were left with the governor
    for method in ["unpause", "pause"] {
        let reason = test.execute_expect_failure(
            call!(test, stable_coin_component, method, args![], &admin_key),
            vec![admin_proof.clone()],
        );
        assert_reject_reason(&reason, "Access Denied");
    }
    let reason = test.execute_expect_failure(
        test.transaction()
            .set_component_access_rules(
                stable_coin_component,
                governed_by(AccessRule::AllowAll).access_rules(),
            )
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );
    assert_reject_reason(&reason, "Access Denied");

    // The new governor holds them and can assign roles
    let roles = RoleConfig {
        minter: Some(signer_rule(&governor_key)),
        ..governed_by(AccessRule::DenyAll)
    };
    test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "pause", args![])
            .call_method(stable_coin_component, "unpause", args![])
            .set_component_access_rules(stable_coin_component, roles.access_rules())
            .call_method(stable_coin_component, "increase_supply", args![100])
            .build_and_seal(&governor_key),
        vec![governor_proof],
    );
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
    let utxos: Vec<UtxoId> = (0..=50u8).map(|i| UtxoId::from_array([i; 32])).collect();

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

/// The freeze events list every UTXO id, so a full batch has to fit the engine's event size cap.
#[test]
fn it_fits_a_full_freeze_batch_in_an_event() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();
    // These UTXOs do not exist, so the call still fails, but only once the event has been accepted.
    let utxos: Vec<UtxoId> = (0..50u8).map(|i| UtxoId::from_array([i; 32])).collect();

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
        let reason = reason.to_string();
        assert!(!reason.contains("exceeds maximum size"), "{reason}");
        assert!(!reason.contains("Too many UTXOs"), "{reason}");
    }
}

#[test]
fn it_rejects_a_governor_or_role_open_to_everyone() {
    let mut test = TemplateTest::my_crate();
    let (_, admin_proof, admin_key) = test.create_funded_account();
    let governor = signer_rule(&admin_key);

    let cases = [
        (
            governed_by(AccessRule::AllowAll),
            "The governor cannot be open to everyone",
        ),
        (
            governed_by(AccessRule::DenyAll),
            "The governor must be held by someone",
        ),
        (
            RoleConfig {
                treasurer: Some(AccessRule::AllowAll),
                ..governed_by(governor)
            },
            "A role cannot be open to everyone",
        ),
    ];
    for (roles, expected) in cases {
        let reason = test.execute_expect_failure(
            instantiate_transaction(&test, &admin_key, StableCoinConfig::default(), roles),
            vec![admin_proof.clone()],
        );
        assert_reject_reason(&reason, expected);
    }
}

#[test]
fn it_rejects_an_initial_supply_above_max_supply() {
    let mut test = TemplateTest::my_crate();
    let (_, admin_proof, admin_key) = test.create_funded_account();
    let config = StableCoinConfig {
        max_supply: Some(Amount::from(INITIAL_SUPPLY - 1)),
        ..Default::default()
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

#[test]
fn it_emits_an_event_named_after_the_method() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();

    let result = test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "increase_supply", args![100])
            .call_method(stable_coin_component, "pause", args![])
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    let events = &result.finalize.events;
    let mint = events
        .iter()
        .find(|e| e.topic() == "TariStableCoin.increase_supply")
        .expect("no increase_supply event");
    assert!(mint.payload().get("amount").is_some());
    let pause = events
        .iter()
        .find(|e| e.topic() == "TariStableCoin.pause")
        .expect("no pause event");
    assert!(pause.payload().get("tx_signer").is_none());
}

#[test]
fn it_burns_a_utxo_and_reports_the_amount() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
    } = setup();
    // `instantiate_transaction` makes the admin key the resource view key
    let view_key = RistrettoPublicKey::from_secret_key(&admin_key);

    // Move 100 out of the issuer vault into a single stealth UTXO
    let transfer =
        stealth::generate_transfer_data_with_view_key(NO_INPUTS, 100u64, [100], 0, &view_key);
    let result = test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "withdraw", args![100])
            .put_last_instruction_output_on_workspace("funds")
            .stealth_transfer_with_input_bucket(token_resource, transfer.statement, "funds")
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );
    let utxo_id = result
        .finalize
        .result
        .any_accept()
        .unwrap()
        .up_iter()
        .find_map(|(id, _)| id.as_utxo_address().map(|addr| *addr.id()))
        .expect("no UTXO created");
    let supply_before = total_supply(&test, token_resource);

    // The view key holder proves the UTXO's value from its viewable balance
    let utxo = test
        .read_only_state_store()
        .get_utxo(UtxoAddress::new(token_resource, utxo_id))
        .unwrap();
    let viewable_balance: ElgamalVerifiableBalance = utxo
        .output()
        .unwrap()
        .output
        .viewable_balance
        .as_ref()
        .unwrap()
        .try_into()
        .unwrap();
    let proof = generate_elgamal_value_proof(
        &admin_key,
        100,
        &utxo_id.into_commitment_bytes(),
        &viewable_balance,
    );

    let result = test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "burn_utxo",
            args![utxo_id, proof],
            &admin_key
        ),
        vec![admin_proof],
    );

    let burn = result
        .finalize
        .events
        .iter()
        .find(|e| e.topic() == "TariStableCoin.burn_utxo")
        .expect("no burn_utxo event");
    let amount: Amount = burn.payload().get("amount").unwrap().decode().unwrap();
    assert_eq!(amount, Amount::from(100u64));
    assert_eq!(
        total_supply(&test, token_resource),
        supply_before - Amount::from(100u64)
    );
}

#[test]
fn it_routes_rule_changes_through_the_template_when_template_managed() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup_with(
        StableCoinConfig {
            template_managed_rules: true,
            ..Default::default()
        },
        |_, governor| governed_by(governor),
    );
    let (_, minter_proof, minter_key) = test.create_empty_account();
    let (_, governor_proof, governor_key) = test.create_empty_account();
    let roles = Roles {
        minter: Some(signer_rule(&minter_key)),
        ..Default::default()
    };

    // The governor cannot change the rules by instruction
    let reason = test.execute_expect_failure(
        test.transaction()
            .set_component_access_rules(stable_coin_component, roles.access_rules())
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "rules updater is TemplateManaged");
    let reason = test.execute_expect_failure(
        test.transaction()
            .set_component_owner_rule(
                stable_coin_component,
                OwnerRule::ByAccessRule(signer_rule(&governor_key)),
            )
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "rules updater is TemplateManaged");

    // Only the governor changes them through the template
    let set_roles = |test: &TemplateTest, key: &RistrettoSecretKey| {
        call!(
            test,
            stable_coin_component,
            "set_roles",
            args![roles.clone()],
            key
        )
    };
    let set_governor = |test: &TemplateTest, key: &RistrettoSecretKey| {
        call!(
            test,
            stable_coin_component,
            "set_governor",
            args![signer_rule(&governor_key)],
            key
        )
    };
    for transaction in [
        set_roles(&test, &minter_key),
        set_governor(&test, &minter_key),
    ] {
        let reason = test.execute_expect_failure(transaction, vec![minter_proof.clone()]);
        assert_reject_reason(&reason, "Access Denied");
    }

    // Each role is reported, with its new rule or none
    let result =
        test.execute_expect_success(set_roles(&test, &admin_key), vec![admin_proof.clone()]);
    let reported: Vec<(String, Option<AccessRule>)> = result
        .finalize
        .events
        .iter()
        .filter(|e| e.topic() == "TariStableCoin.set_roles")
        .map(|e| {
            (
                e.payload().get("role").unwrap().decode().unwrap(),
                e.payload().get("rule").unwrap().decode().unwrap(),
            )
        })
        .collect();
    let expected = [
        ("minter", roles.minter.clone()),
        ("burner", None),
        ("treasurer", None),
        ("compliance", None),
        ("pauser", None),
    ];
    assert_eq!(reported.len(), expected.len());
    for ((role, rule), (expected_role, expected_rule)) in reported.iter().zip(&expected) {
        assert_eq!(role, expected_role);
        assert_eq!(rule, expected_rule);
    }
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

    let result =
        test.execute_expect_success(set_governor(&test, &admin_key), vec![admin_proof.clone()]);
    let event = result
        .finalize
        .events
        .iter()
        .find(|e| e.topic() == "TariStableCoin.set_governor")
        .expect("no set_governor event");
    let new: AccessRule = event.payload().get("new").unwrap().decode().unwrap();
    assert_eq!(new, signer_rule(&governor_key));

    // The old governor no longer governs; the new one does
    let reason = test.execute_expect_failure(set_roles(&test, &admin_key), vec![admin_proof]);
    assert_reject_reason(&reason, "Access Denied");
    test.execute_expect_success(set_roles(&test, &governor_key), vec![governor_proof]);
}

#[test]
fn it_checks_rules_changed_through_the_template() {
    // Without `template_managed_rules` the methods work too, alongside the instructions
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();

    let open_role = Roles {
        treasurer: Some(AccessRule::AllowAll),
        ..Default::default()
    };
    let cases = [
        (
            "set_roles",
            args![open_role],
            "A role cannot be open to everyone",
        ),
        (
            "set_governor",
            args![AccessRule::AllowAll],
            "The governor cannot be open to everyone",
        ),
        (
            "set_governor",
            args![AccessRule::DenyAll],
            "The governor must be held by someone",
        ),
    ];
    for (method, args, expected) in cases {
        let reason = test.execute_expect_failure(
            call!(test, stable_coin_component, method, args, &admin_key),
            vec![admin_proof.clone()],
        );
        assert_reject_reason(&reason, expected);
    }

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "set_roles",
            args![Roles::default()],
            &admin_key
        ),
        vec![admin_proof],
    );
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

/// A role config that leaves every role with `governor`.
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
    setup_with(StableCoinConfig::default(), roles)
}

fn setup_with(
    config: StableCoinConfig,
    roles: impl FnOnce(&mut TemplateTest, AccessRule) -> RoleConfig,
) -> TestSetup {
    let mut test = TemplateTest::my_crate();
    let (_, admin_proof, admin_key) = test.create_funded_account();
    let roles = roles(&mut test, signer_rule(&admin_key));
    let template = test.get_template_address("TariStableCoin");
    let result = test.execute_expect_success(
        instantiate_transaction(&test, &admin_key, config, roles),
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
