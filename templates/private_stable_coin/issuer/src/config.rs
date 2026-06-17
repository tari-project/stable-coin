// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;
use tari_template_lib::types::Amount;

#[derive(Clone, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub struct StableCoinConfig {
    #[n(0)]
    pub transfer_fee: FeeSpec,
    #[n(1)]
    pub wrapped_exchange_fee: FeeSpec,
    #[n(2)]
    pub default_exchange_limit: Amount,
}

impl Default for StableCoinConfig {
    fn default() -> Self {
        Self {
            wrapped_exchange_fee: FeeSpec::Percentage(1),
            transfer_fee: FeeSpec::Fixed(1u64.into()),
            default_exchange_limit: 1000u64.into(),
        }
    }
}

#[derive(Clone, minicbor::Encode, minicbor::Decode, minicbor::CborLen)]
pub enum FeeSpec {
    #[n(0)]
    Fixed(#[n(0)] Amount),
    #[n(1)]
    Percentage(#[n(0)] u8),
}

impl FeeSpec {
    pub fn calculate_fee(&self, amount: Amount) -> Amount {
        match self {
            Self::Fixed(fee) => *fee,
            Self::Percentage(percentage) => perc_rounded(amount, *percentage),
        }
    }
}

impl fmt::Display for FeeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(fee) => write!(f, "{}", fee),
            Self::Percentage(perc) => write!(f, "{}%", perc),
        }
    }
}

fn perc_rounded<A: Into<Amount>>(v: A, percentage: u8) -> Amount {
    let v = v.into();
    let p = Amount::from(percentage);

    // Compute `v * p / 100` rounding half up, working directly with the integer
    // quotient and remainder. The previous version scaled by 1000 first
    // (`v * 1000 * p`), which overflows `Amount` for large supplies.
    let scaled = v * p;
    let whole = scaled / Amount::ONE_HUNDRED;
    let remainder = scaled - (whole * Amount::ONE_HUNDRED);

    // If the fractional part is >= 0.5 (remainder >= 50), round up.
    if remainder >= 50 {
        whole + Amount::ONE
    } else {
        whole
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perc_round() {
        assert_eq!(perc_rounded(0u64, 5), 0);
        assert_eq!(perc_rounded(100u64, 0), 0);
        assert_eq!(perc_rounded(100u64, 5), 5);
        assert_eq!(perc_rounded(123u64, 5), 6);
        assert_eq!(perc_rounded(130u64, 5), 7);
        assert_eq!(perc_rounded(120u64, 10), 12);
        assert_eq!(perc_rounded(1234560000000u64, 11), 135801600000u64);
    }
}
