// Creates an account for `user` and funds it from the issuer vault. Sign with a key that holds the treasurer role.

fn main() {
    let sc = var!["sc"];
    let user = var!["user"];

    let user_acc = create_account!(user);
    let funds = sc.withdraw(1000000);
    user_acc.deposit(funds);
}
