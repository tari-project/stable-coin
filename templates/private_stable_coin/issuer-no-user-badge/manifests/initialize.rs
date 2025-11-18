use template_xxx as StableCoin;

pub fn main() {
    // Initialize the StableCoin component
    StableCoin::initialize(
        "100000000000000000000000000",
        "STC",
        Metadata("provider_name=StableCoin Inc."),
        8,
        // 32-byte public view key (hex)
        PublicKey("deadbeaf..."),
        true,
    );
}
