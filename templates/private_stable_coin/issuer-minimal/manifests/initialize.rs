use StableCoin;

pub fn main() {
    let addr = new_component_addr!();
    // The issuer's public view key. It reveals every amount of the coin, lets the burner burn any UTXO, and
    // can never be rotated: generate it for this coin and keep its secret offline.
    let view_key = var!["view_key"];
    // A `RoleConfig`: the governor rule is required, every other role follows the governor unless set.
    // Rules name signer keys, e.g. `rule!(m_of_n(2, public_key(a), public_key(b), public_key(c)))`.
    let roles = var!["roles"];
    StableCoin::instantiate(
        addr,
        "100000000000000000000000000",
        "STC",
        Metadata("provider_name=StableCoin Inc."),
        8,
        view_key,
        // Config: None leaves supply uncapped
        None,
        roles,
    );
}
