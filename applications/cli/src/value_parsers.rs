// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use anyhow::bail;
use tari_template_lib::models::{Amount, Metadata};

pub fn amount(s: &str) -> anyhow::Result<Amount> {
    let val: u128 = s.trim().parse()?;
    Ok(Amount::from(val))
}

pub fn metadata(s: &str) -> anyhow::Result<Metadata> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Metadata::new());
    }
    s.split(',').filter(|pair| !pair.trim().is_empty()).fold(
        Ok(Metadata::new()),
        |metadata, pair| {
            let mut metadata = metadata?;
            let (raw_key, raw_value) = pair.split_once('=').ok_or_else(|| {
                anyhow::anyhow!("Invalid metadata entry {pair:?}: expected key=value")
            })?;
            let key = raw_key.trim();
            if key.is_empty() {
                bail!("Invalid metadata entry {pair:?}: key cannot be empty");
            }
            let value = raw_value.trim();
            metadata.insert(key, value);
            Ok(metadata)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata_pairs_and_empty_input() {
        let empty = metadata("").unwrap();
        assert_eq!(empty.get("a"), None);

        let parsed = metadata("name=Tari USD, url=https://tari.com?a=1&b=2,").unwrap();
        assert_eq!(parsed.get("name"), Some("Tari USD"));
        assert_eq!(parsed.get("url"), Some("https://tari.com?a=1&b=2"));

        assert!(metadata("=val").is_err());
        assert!(metadata("no_equals").is_err());
    }
}
