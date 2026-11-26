// Hands the coin to a new governor. Sign with the current governor.
//
// A coin instantiated with `template_managed_rules` rejects this instruction: call its `set_governor` method instead.
//
// This takes effect immediately and nothing checks that anyone holds the new rule: a mistyped key leaves the coin
// without a governor for good.

fn main() {
    let sc = var!["sc"];
    let a = var!["key_a"];
    let b = var!["key_b"];
    let c = var!["key_c"];

    set_owner_rule!(sc, access_rule(m_of_n(2, public_key(a), public_key(b), public_key(c))));
}
