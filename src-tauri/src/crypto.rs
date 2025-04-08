use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use directories::ProjectDirs;
use rand::Rng;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub struct Crypto {
    key: Secret<[u8; 32]>,
}

#[derive(Serialize, Deserialize)]
struct EncryptedData {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

impl Crypto {
    pub fn new() -> Self {
        let key = match Self::load_key() {
            Some(k) => k,
            None => Self::generate_and_save_key(),
        };
        Self { key: Secret::new(key) }
    }

    fn get_key_path() -> PathBuf {
        let proj_dirs = ProjectDirs::from("com", "secondbrian", "diary")
            .expect("Failed to get project directories");
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir).expect("Failed to create data directory");
        data_dir.join("encryption.key")
    }

    fn load_key() -> Option<[u8; 32]> {
        let key_path = Self::get_key_path();
        if !key_path.exists() {
            return None;
        }

        let mut file = File::open(key_path).ok()?;
        let mut key = [0u8; 32];
        file.read_exact(&mut key).ok()?;
        Some(key)
    }

    fn generate_and_save_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill(&mut key);

        let key_path = Self::get_key_path();
        let mut file = File::create(key_path).expect("Failed to create key file");
        file.write_all(&key).expect("Failed to write key to file");

        key
    }

    pub fn encrypt(&self, data: &str) -> String {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.expose_secret()));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        
        let ciphertext = cipher
            .encrypt(&nonce, data.as_bytes())
            .expect("Encryption failed");

        let encrypted_data = EncryptedData {
            nonce: nonce.to_vec(),
            ciphertext,
        };

        serde_json::to_string(&encrypted_data).expect("Failed to serialize encrypted data")
    }

    pub fn decrypt(&self, encrypted_data_str: &str) -> String {
        let encrypted_data: EncryptedData =
            serde_json::from_str(encrypted_data_str).expect("Failed to deserialize encrypted data");

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.expose_secret()));
        let nonce = Nonce::from_slice(&encrypted_data.nonce);

        let plaintext = cipher
            .decrypt(nonce, encrypted_data.ciphertext.as_ref())
            .expect("Decryption failed");

        String::from_utf8(plaintext).expect("Invalid UTF-8")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let crypto = Crypto::new();
        let original = "This is a secret message";
        let encrypted = crypto.encrypt(original);
        let decrypted = crypto.decrypt(&encrypted);
        assert_eq!(original, decrypted);
    }
} 