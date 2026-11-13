use ootle_byte_type::ToByteType;
// The template crate is a `cdylib`, so integration tests pull its modules in by path.
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
        test.transaction()
            .call_method(stable_coin_component, "increase_supply", args![123])
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
            .call_method(stable_coin_component, "decrease_supply", args![456])
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
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    // Allow Alice and Bob to transact, and provision funds in Alice's account
    test.execute_expect_success(
        test.transaction()
            // Create user badges for Alice and Bob
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![123, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![456, bob_account],
            )
            .put_last_instruction_output_on_workspace("bob_badge")
            .call_method(stable_coin_component, "withdraw", args![1234])
            .put_last_instruction_output_on_workspace("funds")
            // Deposit badges and funds
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(bob_account, "deposit", args![Workspace("bob_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Alice sends to Bob (both have user badges, so this should succeed)
    test.execute_expect_success(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .authorize_proof("proof")
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
fn it_prevents_deposit_without_user_badge() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();

    let (unauthorized_account, _, _) = test.create_empty_account();

    // Try to withdraw from component vault and deposit to an account without a user badge
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(stable_coin_component, "withdraw", args![100])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(unauthorized_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    assert_reject_reason(&reason, "does not have a vault for the resource");
}

#[test]
fn it_pauses_deposits() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    // Setup: create users and fund Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![2, bob_account],
            )
            .put_last_instruction_output_on_workspace("bob_badge")
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(bob_account, "deposit", args![Workspace("bob_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Pause the token
    test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "pause", args![])
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Now Alice tries to send to Bob - should fail because token is paused
    let reason = test.execute_expect_failure(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .authorize_proof("proof")
            .call_method(alice_account, "withdraw", args![token_resource, 100])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(bob_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&alice_key),
        vec![alice_proof],
    );

    assert_reject_reason(&reason, "Token is paused");
}

#[test]
fn it_blacklists_and_removes_from_blacklist() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    // Setup: create users and fund Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![2, bob_account],
            )
            .put_last_instruction_output_on_workspace("bob_badge")
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(bob_account, "deposit", args![Workspace("bob_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Get Alice's user badge vault ID via the Account model
    let alice_account_state = test
        .read_only_state_store()
        .get_account(alice_account)
        .unwrap();
    let alice_badge_vault_id = alice_account_state
        .get_vault_by_resource(&user_badge_resource)
        .unwrap()
        .vault_id();

    // Blacklist Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "blacklist_user",
                args![alice_badge_vault_id, 1u64],
            )
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Verify Alice's user badge has been recalled (balance should be 0)
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&user_badge_resource).unwrap().balance(), 0);

    // Alice can no longer deposit tokens (no user badge)
    // This tests that the auth hook rejects deposits for blacklisted users
    let reason = test.execute_expect_failure(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .authorize_proof("proof")
            .call_method(alice_account, "withdraw", args![token_resource, 100])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(bob_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&alice_key),
        vec![alice_proof.clone()],
    );
    // Alice can't even create a proof because the badge was recalled
    assert_reject_reason(&reason, "");

    // Remove Alice from the blacklist and re-deposit her badge
    test.execute_expect_success(
        test.transaction()
            .call_method(stable_coin_component, "remove_from_blacklist", args![1u64])
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Verify Alice has her badge back
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&user_badge_resource).unwrap().balance(), 1);
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

    // Setup: create user and fund Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![500])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Verify Alice has 500 tokens
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 500);

    // Recall 200 tokens from Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "recall_revealed_tokens",
                args![1u64, alice_account, 200],
            )
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Alice should now have 300 tokens
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 300);
}

#[test]
fn it_deposits_to_component_vault() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        token_resource,
        ..
    } = setup();

    let (alice_account, _, _) = test.create_empty_account();

    // Setup: create user and fund Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![500])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    let resource_before = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap();
    let supply_before = resource_before.total_supply().unwrap();

    // Deposit tokens back into the component vault (admin withdraws from Alice and deposits back)
    test.execute_expect_success(
        test.transaction()
            // Recall tokens from Alice's account
            .call_method(
                stable_coin_component,
                "recall_revealed_tokens",
                args![1u64, alice_account, 200],
            )
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Supply should remain the same (tokens moved to component vault, not burned)
    let resource_after = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap();
    assert_eq!(resource_after.total_supply().unwrap(), supply_before);
}

#[test]
fn it_sets_config_transfer_fee() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        ..
    } = setup();

    // Set fixed transfer fee
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "set_config_transfer_fee_fixed",
                args![10],
            )
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Set percentage transfer fee
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "set_config_transfer_fee_percentage",
                args![5u8],
            )
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Setting percentage > 100 should fail
    let reason = test.execute_expect_failure(
        test.transaction()
            .call_method(
                stable_coin_component,
                "set_config_transfer_fee_percentage",
                args![101u8],
            )
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
        admin_proof,
        admin_key,
        user_badge_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();

    // Create Alice as a user
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );

    // Alice (user) tries to increase supply - should be denied
    let reason = test.execute_expect_failure(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .authorize_proof("proof")
            .call_method(stable_coin_component, "increase_supply", args![100])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&alice_key),
        vec![alice_proof.clone()],
    );

    assert_reject_reason(&reason, "Access Denied");

    // Alice (user) tries to create a new user - should be denied
    let (bob_account, _, _) = test.create_empty_account();
    let reason = test.execute_expect_failure(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .authorize_proof("proof")
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![2, bob_account],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&alice_key),
        vec![alice_proof],
    );

    assert_reject_reason(&reason, "Access Denied");
}

#[test]
fn it_exchanges_stable_for_wrapped_tokens() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, _alice_key) = test.create_empty_account();

    // Setup: create user and fund Alice with 1000 tokens
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Alice exchanges 100 stable tokens for wrapped tokens
    // Default exchange fee is 1%, so she should get 99 wrapped tokens
    // Signed by the admin key, which holds the treasurer role
    test.execute_expect_success(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("user_proof")
            .authorize_proof("user_proof")
            .call_method(alice_account, "withdraw", args![token_resource, 100])
            .put_last_instruction_output_on_workspace("stable_tokens")
            .call_method(
                stable_coin_component,
                "exchange_stable_for_wrapped_tokens",
                args![Workspace("user_proof"), Workspace("stable_tokens")],
            )
            .put_last_instruction_output_on_workspace("wrapped_tokens")
            .call_method(alice_account, "deposit", args![Workspace("wrapped_tokens")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof, alice_proof],
    );

    // Alice should have 900 stable tokens remaining
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 900);

    // Alice should have wrapped tokens (99 after 1% fee)
    // Find the wrapped token resource (it's a new resource, not the stable token)
    let wrapped_resource = alice_vaults
        .iter()
        .find(|(addr, vault)| {
            **addr != token_resource && **addr != user_badge_resource && vault.balance() > 0
        })
        .map(|(addr, _)| *addr)
        .expect("Alice should have wrapped tokens");

    assert_eq!(alice_vaults.get(&wrapped_resource).unwrap().balance(), 99);
}

#[test]
fn it_exchanges_wrapped_for_stable_tokens() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, _alice_key) = test.create_empty_account();

    // Setup: create user and fund Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // First exchange stable for wrapped (signed by the admin key, which holds the treasurer role)
    test.execute_expect_success(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("user_proof")
            .authorize_proof("user_proof")
            .call_method(alice_account, "withdraw", args![token_resource, 100])
            .put_last_instruction_output_on_workspace("stable_tokens")
            .call_method(
                stable_coin_component,
                "exchange_stable_for_wrapped_tokens",
                args![Workspace("user_proof"), Workspace("stable_tokens")],
            )
            .put_last_instruction_output_on_workspace("wrapped_tokens")
            .call_method(alice_account, "deposit", args![Workspace("wrapped_tokens")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone(), alice_proof.clone()],
    );

    // Find the wrapped token resource
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    let wrapped_resource = alice_vaults
        .iter()
        .find(|(addr, vault)| {
            **addr != token_resource && **addr != user_badge_resource && vault.balance() > 0
        })
        .map(|(addr, _)| *addr)
        .expect("Alice should have wrapped tokens");
    let wrapped_balance = alice_vaults.get(&wrapped_resource).unwrap().balance();

    // Now exchange wrapped tokens back to stable tokens
    test.execute_expect_success(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("user_proof")
            .authorize_proof("user_proof")
            .call_method(
                alice_account,
                "withdraw",
                args![wrapped_resource, wrapped_balance],
            )
            .put_last_instruction_output_on_workspace("wrapped_tokens")
            .call_method(
                stable_coin_component,
                "exchange_wrapped_for_stable_tokens",
                args![Workspace("user_proof"), Workspace("wrapped_tokens")],
            )
            .put_last_instruction_output_on_workspace("stable_tokens")
            .call_method(alice_account, "deposit", args![Workspace("stable_tokens")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof, alice_proof],
    );

    // Alice should have 900 + 99 = 999 stable tokens (lost 1 to fee in the first exchange)
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 999);

    // All wrapped tokens should be burned
    assert_eq!(alice_vaults.get(&wrapped_resource).unwrap().balance(), 0);
}

#[test]
fn it_enforces_exchange_limit() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, _alice_key) = test.create_empty_account();

    // Setup: create user and fund Alice with more than the default exchange limit (1000)
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![2000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Try to exchange more than the default limit (1000) - should fail
    let reason = test.execute_expect_failure(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("user_proof")
            .authorize_proof("user_proof")
            .call_method(alice_account, "withdraw", args![token_resource, 1001])
            .put_last_instruction_output_on_workspace("stable_tokens")
            .call_method(
                stable_coin_component,
                "exchange_stable_for_wrapped_tokens",
                args![Workspace("user_proof"), Workspace("stable_tokens")],
            )
            .put_last_instruction_output_on_workspace("wrapped_tokens")
            .call_method(alice_account, "deposit", args![Workspace("wrapped_tokens")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof, alice_proof],
    );

    assert_reject_reason(&reason, "Exchange limit exceeded");
}

#[test]
fn it_sets_user_exchange_limit() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, _alice_key) = test.create_empty_account();

    // Setup: create user and fund Alice
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![5000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Increase Alice's exchange limit to 3000
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "set_user_exchange_limit",
                args![1u64, 3000],
            )
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // Alice can now exchange 2000 (within new limit of 3000)
    // Signed by the admin key, which holds the treasurer role
    test.execute_expect_success(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("user_proof")
            .authorize_proof("user_proof")
            .call_method(alice_account, "withdraw", args![token_resource, 2000])
            .put_last_instruction_output_on_workspace("stable_tokens")
            .call_method(
                stable_coin_component,
                "exchange_stable_for_wrapped_tokens",
                args![Workspace("user_proof"), Workspace("stable_tokens")],
            )
            .put_last_instruction_output_on_workspace("wrapped_tokens")
            .call_method(alice_account, "deposit", args![Workspace("wrapped_tokens")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof, alice_proof],
    );

    // Alice should have 3000 stable tokens remaining (5000 - 2000)
    let alice_vaults = test
        .read_only_state_store()
        .get_vaults_for_account(alice_account)
        .unwrap();
    assert_eq!(alice_vaults.get(&token_resource).unwrap().balance(), 3000);
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

    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "set_config_max_supply",
            args![Some(max_supply)],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );

    // Up to the cap
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![100],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    let supply = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap()
        .total_supply()
        .unwrap();
    assert_eq!(supply, max_supply);

    // Over the cap
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![1],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "Minting would exceed the max supply");

    // The cap cannot drop below the current supply
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "set_config_max_supply",
            args![Some(max_supply - Amount::ONE)],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "The max supply is below the current supply");

    // Removing the cap restores unlimited minting
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "set_config_max_supply",
            args![None::<Amount>],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![1],
            &admin_key
        ),
        vec![admin_proof],
    );
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
fn it_counts_wrapped_tokens_toward_max_supply() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
        ..
    } = setup();
    let (alice_account, alice_proof, _) = test.create_empty_account();

    // Alice exchanges 100 coins for 99 wrapped tokens; the 1 coin fee stays in the treasury
    test.execute_expect_success(
        test.transaction()
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![1, alice_account],
            )
            .put_last_instruction_output_on_workspace("alice_badge")
            .call_method(stable_coin_component, "withdraw", args![1000])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(alice_account, "deposit", args![Workspace("alice_badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );
    test.execute_expect_success(
        test.transaction()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("user_proof")
            .authorize_proof("user_proof")
            .call_method(alice_account, "withdraw", args![token_resource, 100])
            .put_last_instruction_output_on_workspace("stable_tokens")
            .call_method(
                stable_coin_component,
                "exchange_stable_for_wrapped_tokens",
                args![Workspace("user_proof"), Workspace("stable_tokens")],
            )
            .put_last_instruction_output_on_workspace("wrapped_tokens")
            .call_method(alice_account, "deposit", args![Workspace("wrapped_tokens")])
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone(), alice_proof],
    );
    let coin_supply = test
        .read_only_state_store()
        .get_resource(&token_resource)
        .unwrap()
        .total_supply()
        .unwrap();
    assert_eq!(coin_supply, Amount::from(INITIAL_SUPPLY - 99));

    // The cap counts the 99 wrapped tokens alongside the coin
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "set_config_max_supply",
            args![Some(coin_supply)],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    assert_reject_reason(&reason, "The max supply is below the current supply");
    test.execute_expect_success(
        call!(
            test,
            stable_coin_component,
            "set_config_max_supply",
            args![Some(Amount::from(INITIAL_SUPPLY))],
            &admin_key
        ),
        vec![admin_proof.clone()],
    );
    let reason = test.execute_expect_failure(
        call!(
            test,
            stable_coin_component,
            "increase_supply",
            args![1],
            &admin_key
        ),
        vec![admin_proof],
    );
    assert_reject_reason(&reason, "Minting would exceed the max supply");
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

struct TestSetup {
    test: TemplateTest,
    stable_coin_component: ComponentAddress,
    admin_proof: NonFungibleAddress,
    admin_key: RistrettoSecretKey,
    user_badge_resource: ResourceAddress,
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
        user_manager: None,
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
    admin_key: &RistrettoSecretKey,
    roles: RoleConfig,
) -> Transaction {
    let template = test.get_template_address("TariStableCoin");
    let mut metadata = Metadata::new();
    metadata
        .insert("provider_name", "Stable coinz 4 U")
        .insert("collateralized_by", "Z$")
        .insert("issuing_authority", "Bank of Silly Walks")
        .insert("issued_at", "2023-01-01");

    let view_key = RistrettoPublicKey::from_secret_key(admin_key).to_byte_type();
    test.transaction()
        .call_function(
            template,
            "instantiate",
            args![INITIAL_SUPPLY, "SC4U", metadata, 8, view_key, true, roles],
        )
        .build_and_seal(admin_key)
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
        instantiate_transaction(&test, &admin_key, roles),
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
    let user_badge_resource = indexed
        .get_value("$.2")
        .unwrap()
        .expect("user_auth_manager not found");

    let vault = test
        .read_only_state_store()
        .get_vault(&token_vault)
        .unwrap();
    let token_resource = *vault.resource_address();

    TestSetup {
        test,
        stable_coin_component,
        admin_proof,
        admin_key,
        user_badge_resource,
        token_resource,
    }
}
