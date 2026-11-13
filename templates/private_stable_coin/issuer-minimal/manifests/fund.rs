use StableCoin;

// Sign with a key that holds the treasurer role.
pub fn main() {
    let account = var!["account"];
    let sc = var!["sc"];
    let coins = sc.withdraw("1000000000");
    account.deposit(coins);
}
