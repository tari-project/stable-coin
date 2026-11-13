// Registers a new user and transfers them some funds. Sign with keys that hold the user manager and treasurer
// roles.

fn main() {
    let sc = var!["sc"];
    let user = var!["user"];
    let badge = sc.create_new_user(1, user);
    user.deposit(badge);
    let funds = sc.withdraw(1000000);
    user.deposit(funds);
}
