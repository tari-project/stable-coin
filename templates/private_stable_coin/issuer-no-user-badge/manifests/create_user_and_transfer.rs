// Registers a new user and transfers them some funds. Sign with keys that hold the user manager and treasurer
// roles.

fn main() {
    let sc = var!["sc"];
    let user = var!["user"];

    let user_acc = create_account!(user);
    let badge = sc.create_new_user(1, user_acc);
    user_acc.deposit(badge);
    let funds = sc.withdraw(1000000);
    user_acc.deposit(funds);
}
