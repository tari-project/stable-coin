// Replaces who holds each role. Sign with the governor.
//
// The instruction replaces every method rule at once: list each role that is not left with the governor, with all
// of its methods. A method without a rule can only be called by the governor, so to leave a role with the governor
// delete its lines.

fn main() {
    let sc = var!["sc"];
    let minter = var!["minter"];
    let burner = var!["burner"];
    let treasurer = var!["treasurer"];
    let compliance = var!["compliance"];
    let pauser = var!["pauser"];

    set_access_rules!(
        sc,
        increase_supply = public_key(minter),
        decrease_supply = public_key(burner),
        burn_utxo = public_key(burner),
        withdraw = public_key(treasurer),
        deposit = public_key(treasurer),
        recall_revealed_tokens = public_key(compliance),
        freeze_utxos = public_key(compliance),
        unfreeze_utxos = public_key(compliance),
        pause = public_key(pauser),
    );
}
