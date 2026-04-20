// This manifest creates a new user in the private stable coin system and transfers them some funds.

fn main() {
    let sc = var!["sc"];
    let resx = var!["resx"];
    let user = var!["user"];
    let account = var!["account"];

    let proof = account.create_proof_by_amount(Address(resx), 1);
    let user_acc = create_account!(user);
    let badge = sc.create_new_user(1, user_acc);
    user_acc.deposit(badge);
    let funds = sc.withdraw(1000000);
    user_acc.deposit(funds);

    drop_all_proofs!()
}
