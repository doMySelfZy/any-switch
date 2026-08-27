use crate::error::{AppError, AppResult};
use crate::paths::{master_key_path, set_secret_permissions};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
pub struct Crypto {
    key: [u8; 32],
}

impl Crypto {
    #[cfg(test)]
    pub fn from_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn load_or_create() -> AppResult<Self> {
        let path = master_key_path()?;
        if path.exists() {
            let bytes = fs::read(&path)?;
            if bytes.len() != 32 {
                return Err(AppError::new(
                    "master_key_missing",
                    "master.key is invalid (expected 32 bytes)",
                ));
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(Self { key })
        } else {
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, key)?;
            set_secret_permissions(&path);
            Ok(Self { key })
        }
    }

    pub(crate) fn from_restore_key(bytes: &[u8]) -> AppResult<Self> {
        if bytes.len() != 32 {
            return Err(AppError::new(
                "backup_invalid",
                "backup master.key must contain exactly 32 bytes",
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }

    pub fn ensure_can_decrypt_db(db_has_encrypted_rows: bool) -> AppResult<Self> {
        let path = master_key_path()?;
        if !path.exists() && db_has_encrypted_rows {
            return Err(AppError::new(
                "master_key_missing",
                "Cannot decrypt stored API keys: master.key is missing. Restore master.key+DB from backup, or clear the data directory.",
            ));
        }
        Self::load_or_create()
    }

    pub fn encrypt(&self, plaintext: &str) -> AppResult<String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::new("internal", e.to_string()))?;
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AppError::new("internal", format!("encrypt failed: {e}")))?;
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(B64.encode(out))
    }

    pub fn decrypt(&self, encoded: &str) -> AppResult<String> {
        let raw = B64
            .decode(encoded)
            .map_err(|e| AppError::new("master_key_missing", format!("decode failed: {e}")))?;
        if raw.len() < 13 {
            return Err(AppError::new("master_key_missing", "ciphertext too short"));
        }
        let (nonce_bytes, ct) = raw.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| AppError::new("internal", e.to_string()))?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = cipher.decrypt(nonce, ct).map_err(|_| {
            AppError::new(
                "master_key_missing",
                "Failed to decrypt API key (wrong or missing master.key)",
            )
        })?;
        String::from_utf8(plain).map_err(|e| AppError::new("internal", format!("utf8 error: {e}")))
    }
}

pub fn key_fingerprint(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn key_prefix(api_key: &str) -> String {
    if api_key.is_empty() {
        return String::new();
    }
    if api_key.len() <= 8 {
        return format!("{}…", &api_key[..api_key.len().min(2)]);
    }
    format!("{}…{}", &api_key[..4], &api_key[api_key.len() - 4..])
}

pub fn redact(value: &str) -> String {
    key_prefix(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_stable() {
        assert_eq!(key_fingerprint("sk-abc"), key_fingerprint("sk-abc"));
        assert_ne!(key_fingerprint("a"), key_fingerprint("b"));
    }
}
