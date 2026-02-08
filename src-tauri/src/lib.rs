// src-tauri/src/lib.rs
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{ChaCha20Poly1305, Key};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
enum EncryptionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Invalid algorithm")]
    InvalidAlgorithm,
}

impl serde::Serialize for EncryptionError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Algorithm {
    AES256GCM,
    ChaCha20Poly1305,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedMetadata {
    algorithm: Algorithm,
    salt: String,
    nonce: String,
    original_filename: String,
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], EncryptionError> {
    let argon2 = Argon2::default();
    let salt_string =
        SaltString::encode_b64(salt).map_err(|e| EncryptionError::Encryption(e.to_string()))?;

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt_string)
        .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

    let hash = password_hash
        .hash
        .ok_or_else(|| EncryptionError::Encryption("Failed to get hash".to_string()))?;

    let mut key = [0u8; 32];
    key.copy_from_slice(&hash.as_bytes()[..32]);
    Ok(key)
}

#[tauri::command]
async fn encrypt_file(
    file_path: String,
    password: String,
    algorithm: Algorithm,
    output_dir: String,
) -> Result<String, EncryptionError> {
    let file_data = fs::read(&file_path)?;

    let salt = rand::random::<[u8; 16]>();
    let nonce_bytes = rand::random::<[u8; 12]>();

    let key = derive_key(&password, &salt)?;

    let encrypted_data = match algorithm {
        Algorithm::AES256GCM => {
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|e| EncryptionError::Encryption(e.to_string()))?;
            let nonce = Nonce::from_slice(&nonce_bytes);
            cipher.encrypt(nonce, file_data.as_ref())
                .map_err(|e| EncryptionError::Encryption(e.to_string()))?
        }
        Algorithm::ChaCha20Poly1305 => {
            let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
            let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
            cipher.encrypt(nonce, file_data.as_ref())
                .map_err(|e| EncryptionError::Encryption(e.to_string()))?
        }
    };

    let original_filename = PathBuf::from(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let metadata = EncryptedMetadata {
        algorithm: algorithm.clone(),
        salt: general_purpose::STANDARD.encode(salt),
        nonce: general_purpose::STANDARD.encode(nonce_bytes),
        original_filename: original_filename.clone(),
    };

    let output_path = PathBuf::from(&output_dir).join(format!("{}.encrypted", original_filename));

    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| EncryptionError::Encryption(e.to_string()))?;
    let metadata_bytes = metadata_json.as_bytes();
    let metadata_len = (metadata_bytes.len() as u32).to_le_bytes();

    let mut output_data = Vec::new();
    output_data.extend_from_slice(&metadata_len);
    output_data.extend_from_slice(metadata_bytes);
    output_data.extend_from_slice(&encrypted_data);

    fs::write(&output_path, output_data)?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn decrypt_file(
    file_path: String,
    password: String,
    output_dir: String,
) -> Result<String, EncryptionError> {
    let file_data = fs::read(&file_path)?;

    if file_data.len() < 4 {
        return Err(EncryptionError::Encryption("Invalid file format".to_string()));
    }

    let metadata_len =
        u32::from_le_bytes([file_data[0], file_data[1], file_data[2], file_data[3]]) as usize;

    if file_data.len() < 4 + metadata_len {
        return Err(EncryptionError::Encryption("Invalid file format".to_string()));
    }

    let metadata_bytes = &file_data[4..4 + metadata_len];
    let metadata: EncryptedMetadata = serde_json::from_slice(metadata_bytes)
        .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

    let encrypted_data = &file_data[4 + metadata_len..];

    let salt = general_purpose::STANDARD
        .decode(&metadata.salt)
        .map_err(|e| EncryptionError::Encryption(e.to_string()))?;
    let nonce_bytes = general_purpose::STANDARD
        .decode(&metadata.nonce)
        .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

    let key = derive_key(&password, &salt)?;

    let decrypted_data = match metadata.algorithm {
        Algorithm::AES256GCM => {
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|e| EncryptionError::Encryption(e.to_string()))?;
            let nonce = Nonce::from_slice(&nonce_bytes);
            cipher.decrypt(nonce, encrypted_data).map_err(|_| {
                EncryptionError::Encryption("Decryption failed - wrong password?".to_string())
            })?
        }
        Algorithm::ChaCha20Poly1305 => {
            let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
            let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
            cipher.decrypt(nonce, encrypted_data).map_err(|_| {
                EncryptionError::Encryption("Decryption failed - wrong password?".to_string())
            })?
        }
    };

    let output_path = PathBuf::from(&output_dir).join(&metadata.original_filename);
    fs::write(&output_path, decrypted_data)?;

    Ok(output_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![encrypt_file, decrypt_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
