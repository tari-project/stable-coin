//! Passphrase-encrypted key files.
//!
//! A key file holds the two secrets of an Ootle address (account and view). They are encrypted
//! with XChaCha20-Poly1305 under a key derived from the passphrase with Argon2id.

use std::{fs, path::Path};

use anyhow::{Context, anyhow, bail};
use argon2::Argon2;
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use ootle_rs::{Network, keys::OotleSecretKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tari_crypto::{ristretto::RistrettoSecretKey, tari_utilities::ByteArray};

const VERSION: u8 = 1;
const PASSPHRASE_ENV: &str = "STABLECOIN_PASSPHRASE";

#[derive(Serialize, Deserialize)]
struct KeyFile {
    version: u8,
    network: String,
    /// Not secret, and not trusted: shown by `key show` without the passphrase.
    address: String,
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// Creates a new random key and writes it to `path`, refusing to overwrite an existing file.
pub fn create(path: &Path, network: Network, passphrase: &str) -> anyhow::Result<OotleSecretKey> {
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    let key = OotleSecretKey::random(network);
    let mut secrets = Vec::with_capacity(64);
    secrets.extend_from_slice(key.account_secret().as_bytes());
    secrets.extend_from_slice(key.view_only_secret().as_bytes());

    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    rand::rng().fill_bytes(&mut salt);
    rand::rng().fill_bytes(&mut nonce);
    let cipher = cipher(passphrase, &salt)?;
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), secrets.as_slice())
        .map_err(|_| anyhow!("Failed to encrypt the key"))?;

    let file = KeyFile {
        version: VERSION,
        network: network.to_string(),
        address: key.to_address().to_string(),
        salt: hex::encode(salt),
        nonce: hex::encode(nonce),
        ciphertext: hex::encode(ciphertext),
    };
    fs::write(path, serde_json::to_string_pretty(&file)?)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    restrict_permissions(path);
    Ok(key)
}

/// Decrypts the key file at `path`.
pub fn open(path: &Path, passphrase: &str) -> anyhow::Result<OotleSecretKey> {
    let file = read(path)?;
    let network: Network = file.network.parse().map_err(|e| anyhow!("{e}"))?;
    let salt = hex::decode(&file.salt).context("Invalid salt")?;
    let nonce = hex::decode(&file.nonce).context("Invalid nonce")?;
    let ciphertext = hex::decode(&file.ciphertext).context("Invalid ciphertext")?;
    if nonce.len() != 24 {
        bail!("Invalid nonce length");
    }
    let secrets = cipher(passphrase, &salt)?
        .decrypt(XNonce::from_slice(&nonce), ciphertext.as_slice())
        .map_err(|_| anyhow!("Wrong passphrase, or the key file is corrupt"))?;
    if secrets.len() != 64 {
        bail!("Unexpected key length");
    }
    let account =
        RistrettoSecretKey::from_canonical_bytes(&secrets[..32]).map_err(|e| anyhow!("{e}"))?;
    let view =
        RistrettoSecretKey::from_canonical_bytes(&secrets[32..]).map_err(|e| anyhow!("{e}"))?;
    Ok(OotleSecretKey::new(network, account, view))
}

/// The address recorded in the key file, read without the passphrase.
pub fn recorded_address(path: &Path) -> anyhow::Result<String> {
    Ok(read(path)?.address)
}

/// Reads the passphrase from `STABLECOIN_PASSPHRASE`, or prompts for it.
pub fn passphrase(confirm: bool) -> anyhow::Result<String> {
    if let Ok(passphrase) = std::env::var(PASSPHRASE_ENV) {
        return Ok(passphrase);
    }
    let passphrase = rpassword::prompt_password("Key passphrase: ")?;
    if confirm && passphrase != rpassword::prompt_password("Repeat passphrase: ")? {
        bail!("The passphrases do not match");
    }
    Ok(passphrase)
}

fn read(path: &Path) -> anyhow::Result<KeyFile> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let file: KeyFile = serde_json::from_str(&contents).context("Not a key file")?;
    if file.version != VERSION {
        bail!("Unsupported key file version {}", file.version);
    }
    Ok(file)
}

fn cipher(passphrase: &str, salt: &[u8]) -> anyhow::Result<XChaCha20Poly1305> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| anyhow!("Key derivation failed: {e}"))?;
    Ok(XChaCha20Poly1305::new((&key).into()))
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut suffix = [0u8; 8];
        rand::rng().fill_bytes(&mut suffix);
        std::env::temp_dir().join(format!("stablecoin-{name}-{}.key", hex::encode(suffix)))
    }

    #[test]
    fn it_round_trips_a_key() {
        let path = temp_path("roundtrip");
        let created = create(&path, Network::LocalNet, "correct horse").unwrap();
        let opened = open(&path, "correct horse").unwrap();
        assert_eq!(created.to_address(), opened.to_address());
        assert_eq!(
            recorded_address(&path).unwrap(),
            created.to_address().to_string()
        );
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn it_rejects_a_wrong_passphrase_and_refuses_to_overwrite() {
        let path = temp_path("wrong");
        create(&path, Network::LocalNet, "right").unwrap();
        assert!(open(&path, "wrong").is_err());
        assert!(create(&path, Network::LocalNet, "right").is_err());
        fs::remove_file(&path).unwrap();
    }
}
