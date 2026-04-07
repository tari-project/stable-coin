use template_xxx as StableCoin;

pub fn main() {
    let addr = new_component_addr!();
    let account = var!["account"];
    // Initialize the StableCoin component
    let badge = StableCoin::instantiate(
        addr,
        "100000000000000000000000000",
        "STC",
        Metadata("provider_name=StableCoin Inc."),
        8,
        // 32-byte public view key (hex)
        PublicKey("6c64d361a661900be82695786a3f9cdbae26f7b216c3d093bea101a309700379"),
        true,
    );
    account.deposit(badge);
}
