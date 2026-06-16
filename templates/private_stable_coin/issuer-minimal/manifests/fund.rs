use StableCoin;

pub fn main() {
    let admin_account = var!["admin_account"];
    let account = var!["account"];
    let sc = var!["sc"];
    let badge_resx = var!["badge"];
    let _proof = admin_account.create_proof_by_amount(badge_resx, 1);
    let coins = sc.withdraw("1000000000");
    account.deposit(coins);
    drop_all_proofs!();
}
