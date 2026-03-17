// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use core::fmt;
use tari_template_lib::types::Amount;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StableCoinConfig {
    pub transfer_fee: FeeSpec,
    pub wrapped_exchange_fee: FeeSpec,
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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum FeeSpec {
    Fixed(Amount),
    Percentage(u8),
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

    // f and b are the division to 3 decimals
    let f = (v * Amount::ONE_THOUSAND) * p / Amount::ONE_HUNDRED;
    let b = v * p / Amount::ONE_HUNDRED;
    let c = f - (b * Amount::ONE_THOUSAND);

    // If the decimal is greater or equal to 0.5, we round up
    if c >= 500 {
        (f / Amount::ONE_THOUSAND) + Amount::ONE
    } else {
        b
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
