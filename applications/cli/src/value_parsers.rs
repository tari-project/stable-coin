// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::Metadata;
use tari_template_lib::types::Amount;

pub fn amount(s: &str) -> anyhow::Result<Amount> {
    let parsed: i64 = s.parse()?;
    let amt = Amount::from(parsed);
    if amt.is_negative() {
        anyhow::bail!("Amount must be non-negative");
    }
    Ok(amt)
}

pub fn metadata(s: &str) -> anyhow::Result<Metadata> {
    if s.trim().is_empty() {
        return Ok(Metadata::new());
    }
    s.split(',').try_fold(Metadata::new(), |mut metadata, pair| {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("Invalid metadata"))?;
        let key = key.trim();
        if key.is_empty() || value.contains('=') {
            anyhow::bail!("Invalid metadata");
        }
        metadata.insert(key, value);
        Ok(metadata)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_rejects_negative_values() {
        assert_eq!(amount("0").unwrap(), Amount::from(0));
        assert_eq!(amount("2000").unwrap(), Amount::from(2000));
        assert!(amount("-1").is_err());
    }

    #[test]
    fn metadata_parses_valid_pairs_and_rejects_malformed_entries() {
        let empty = metadata("").unwrap();
        assert_eq!(empty, Metadata::new());

        let parsed = metadata("name=Tari USD,decimals=6").unwrap();
        assert_eq!(parsed.get("name").map(String::as_str), Some("Tari USD"));
        assert_eq!(parsed.get("decimals").map(String::as_str), Some("6"));

        assert!(metadata("=value").is_err());
        assert!(metadata("   =value").is_err());
        assert!(metadata("key_without_equals").is_err());
        assert!(metadata("k=v=extra").is_err());
    }
}
