// This manifest creates a new user in the private stable coin system and transfers them some funds.

fn main() {
    let sc = var!["sc"];
    let resx = var!["resx"];
    let user = var!["user"];

    let proof = account.create_proof_by_amount(Address(resx), 1);
    let badge = sc.create_new_user(1, user);
    user.deposit(badge);
    let funds = sc.withdraw(1000000);
    user.deposit(funds);

    drop_all_proofs!();
}
