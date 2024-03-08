// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use tari_template_lib::models::{Amount, Metadata};

pub fn amount(s: &str) -> anyhow::Result<Amount> {
    Ok(Amount::new(s.parse()?))
}

pub fn metadata(s: &str) -> anyhow::Result<Metadata> {
    s.split(',').fold(Ok(Metadata::new()), |metadata, pair| {
        let mut metadata = metadata?;
        let mut parts = pair.split('=');
        let key = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid metadata"))?;
        let value = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid metadata"))?;
        metadata.insert(key, value);
        Ok(metadata)
    })
}
