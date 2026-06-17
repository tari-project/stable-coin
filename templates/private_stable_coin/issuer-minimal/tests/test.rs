use ootle_byte_type::ToByteType;
// The template crate is a `cdylib` (so the wasm stays small — adding an `rlib` to
// `crate-type` inflates the wasm by ~47%). Integration tests can't link a cdylib, so
// pull the config module straight into the test crate instead.
#[path = "../src/config.rs"]
#[allow(dead_code)] // not every config helper is exercised by the tests
mod config;
use config::StableCoinConfig;
use tari_template_lib::types::{ComponentAddress, Metadata, NonFungibleAddress, ResourceAddress};
use tari_template_test_tooling::TemplateTest;
use tari_template_test_tooling::crypto::{PublicKey, RistrettoPublicKey, RistrettoSecretKey};
use tari_template_test_tooling::support::assert_error::assert_reject_reason;
use tari_template_test_tooling::transaction::args;

const INITIAL_SUPPLY: u64 = 1_000_000_000_000_000u64;

#[test]
fn it_increases_and_decreases_supply() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        token_resource,
        ..
    } = setup();

    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "increase_supply", args![123])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    let resource = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap();

    assert_eq!(resource.total_supply().unwrap(), INITIAL_SUPPLY + 123);

    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "decrease_supply", args![456])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    let resource = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap();

    assert_eq!(resource.total_supply().unwrap(), INITIAL_SUPPLY + 123 - 456);
}

#[test]
fn it_allows_users_to_transact() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    // Allow Alice to transact and provision funds in her account
    test.execute_expect_success(
        test.transaction()
            // Auth
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "withdraw", args![1234])
            .put_last_instruction_output_on_workspace("funds")
            // Deposit badge and funds into Alice's account
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Alice to Bob should succeed (anyone can receive tokens without a badge)
    test.execute_expect_success(
        test.transaction()
            .call_method(alice_account, "withdraw", args![token_resource, 456])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(bob_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&alice_key),
        vec![alice_proof.clone()],
    );

    let vaults = test
        .read_only_state_store()
        .get_vaults_for_account(bob_account)
        .unwrap();
    assert_eq!(vaults.get(&token_resource).unwrap().balance(), 456);
}

#[test]
fn it_allows_anyone_to_receive_tokens_without_badge() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    // Fund Alice directly (no user badge needed)
    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Alice sends to Bob - no user badge needed for either party
    test.execute_expect_success(
        test.transaction()
            .call_method(alice_account, "withdraw", args![token_resource, 456])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(bob_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
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
fn it_creates_new_admin() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        ..
    } = setup();

    let (new_admin_account, _, _) = test.create_empty_account();

    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_component,
                "create_new_admin",
                args!["employee_42"],
            )
            .put_last_instruction_output_on_workspace("new_admin_badge")
            .call_method(
                new_admin_account,
                "deposit",
                args![Workspace("new_admin_badge")],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    let vaults = test
        .read_only_state_store()
        .get_vaults_for_account(new_admin_account)
        .unwrap();
    assert_eq!(vaults.get(&admin_badge_resource).unwrap().balance(), 1);
}

#[test]
fn it_recalls_tokens_from_user() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, _, _) = test.create_empty_account();

    // Create user and fund Alice
    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "withdraw", args![500])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    let account = test
        .read_only_state_store()
        .get_account(alice_account)
        .unwrap();
    let vault_id = account
        .get_vault_by_resource(&token_resource)
        .unwrap()
        .vault_id();
    let alice_vault = test.read_only_state_store().get_vault(&vault_id).unwrap();
    assert_eq!(alice_vault.balance(), 500);

    // Recall 200 tokens from Alice
    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_component,
                "recall_revealed_tokens",
                args![vault_id, 200],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 300);

    // Total supply unchanged (tokens moved to component vault, not burned)
    let resource = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap();
    assert_eq!(resource.total_supply().unwrap(), INITIAL_SUPPLY);
}

#[test]
fn it_sets_config_transfer_fee() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        ..
    } = setup();

    // Set fixed transfer fee
    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_component,
                "set_config_transfer_fee_fixed",
                args![10],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Set percentage transfer fee
    test.execute_expect_success(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_component,
                "set_config_transfer_fee_percentage",
                args![5u8],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Setting percentage > 100 should fail
    let reason = test.execute_expect_failure(
        test.transaction()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_component,
                "set_config_transfer_fee_percentage",
                args![101u8],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    assert_reject_reason(&reason, "Percentage fee must be between 0 and 100");
}

#[test]
fn it_prevents_non_admin_from_calling_admin_methods() {
    let TestSetup {
        mut test,
        stable_coin_component,
        ..
    } = setup();

    let (_alice_account, alice_proof, alice_key) = test.create_empty_account();

    // Alice (non-admin) tries to increase supply - should be denied
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(stable_coin_component, "increase_supply", args![100])
            .build_and_seal(&alice_key),
        vec![alice_proof.clone()],
    );

    assert_reject_reason(&reason, "Access Denied");
}

struct TestSetup {
    test: TemplateTest,
    stable_coin_component: ComponentAddress,
    admin_account: ComponentAddress,
    admin_proof: NonFungibleAddress,
    admin_key: RistrettoSecretKey,
    admin_badge_resource: ResourceAddress,
    token_resource: ResourceAddress,
}

fn setup() -> TestSetup {
    let mut test = TemplateTest::my_crate();
    let (admin_account, admin_proof, admin_key) = test.create_funded_account();
    let template = test.get_template_address("TariStableCoin");
    let mut metadata = Metadata::new();
    metadata
        .insert("provider_name", "Stable coinz 4 U")
        .insert("collateralized_by", "Z$")
        .insert("issuing_authority", "Bank of Silly Walks")
        .insert("issued_at", "2023-01-01");

    let view_key = RistrettoPublicKey::from_secret_key(&admin_key).to_byte_type();
    let result = test.execute_expect_success(
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
                    StableCoinConfig::default()
                ],
            )
            .put_last_instruction_output_on_workspace("admin_badge")
            .call_method(admin_account, "deposit", args![Workspace("admin_badge")])
            .build_and_seal(&admin_key),
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

    let indexed = test
        .read_only_state_store()
        .inspect_component(stable_coin_component)
        .unwrap();

    // Component state is encoded with minicbor as a positional array, so look up fields by
    // their #[n(N)] index rather than by name.
    let token_vault = indexed
        .get_value("$.1")
        .unwrap()
        .expect("token_vault not found");
    let admin_badge_resource = indexed
        .get_value("$.2")
        .unwrap()
        .expect("admin_auth_manager not found");

    let vault = test
        .read_only_state_store()
        .get_vault(&token_vault)
        .unwrap();
    let token_resource = *vault.resource_address();

    TestSetup {
        test,
        stable_coin_component,
        admin_account,
        admin_proof,
        admin_key,
        admin_badge_resource,
        token_resource,
    }
}
