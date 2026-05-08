use anyhow::{anyhow, bail, Context, Result};
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const VAULT_KEY_FILE_NAME: &str = "vault.key";
const VAULT_KEY_BYTES: usize = 32;
const ENCRYPTED_VALUE_VERSION: &str = "v1";

#[derive(Debug, Clone)]
pub struct VaultSecretRef {
    pub id: String,
    pub redacted: String,
}

#[derive(Debug, Clone)]
pub struct VaultItemView {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub encrypted_value: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
    pub metadata: Value,
}

pub fn vault_key_path_for_db(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join(VAULT_KEY_FILE_NAME)
}

pub fn ensure_vault_key(db_path: &Path) -> Result<PathBuf> {
    let path = vault_key_path_for_db(db_path);
    if path.exists() {
        let key = read_vault_key(&path)?;
        if key.len() != VAULT_KEY_BYTES {
            bail!("Local appliance vault key is malformed.");
        }
        restrict_key_file_permissions(&path)?;
        return Ok(path);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let key = ChaCha20Poly1305::generate_key(&mut OsRng);
    #[cfg(unix)]
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .with_context(|| {
            format!(
                "Failed to create local appliance vault key at {}",
                path.display()
            )
        })?;
    #[cfg(not(unix))]
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| {
            format!(
                "Failed to create local appliance vault key at {}",
                path.display()
            )
        })?;
    file.write_all(hex_encode(&key).as_bytes())?;
    file.write_all(b"\n")?;
    restrict_key_file_permissions(&path)?;
    Ok(path)
}

pub fn store_secret(
    db_path: &Path,
    connection: &Connection,
    kind: &str,
    label: &str,
    plaintext: &str,
    existing_ref: Option<&str>,
    metadata: Value,
) -> Result<VaultSecretRef> {
    let normalized = plaintext.trim();
    if normalized.is_empty() {
        bail!("Vault secret value is required.");
    }
    let encrypted_value = encrypt_secret(db_path, normalized.as_bytes())?;
    let now = Utc::now().to_rfc3339();
    let id = existing_ref
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("vault_item_{}", Uuid::new_v4()));
    connection.execute(
        "INSERT INTO vault_items (
            id, kind, label, encrypted_value, created_at, updated_at, last_used_at, metadata_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL, ?6)
         ON CONFLICT(id) DO UPDATE SET
            kind = excluded.kind,
            label = excluded.label,
            encrypted_value = excluded.encrypted_value,
            updated_at = excluded.updated_at,
            metadata_json = excluded.metadata_json",
        params![id, kind, label, encrypted_value, now, metadata.to_string()],
    )?;
    Ok(VaultSecretRef {
        id,
        redacted: redact_secret(normalized),
    })
}

pub fn decrypt_secret(db_path: &Path, connection: &Connection, secret_ref: &str) -> Result<String> {
    let encrypted_value: String = connection
        .query_row(
            "SELECT encrypted_value FROM vault_items WHERE id = ?1",
            [secret_ref],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Vault item was not found."))?;
    let plaintext = decrypt_value(db_path, &encrypted_value)?;
    connection.execute(
        "UPDATE vault_items SET last_used_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), secret_ref],
    )?;
    String::from_utf8(plaintext).context("Vault item is not valid UTF-8")
}

pub fn get_vault_item(connection: &Connection, secret_ref: &str) -> Result<Option<VaultItemView>> {
    connection
        .query_row(
            "SELECT id, kind, label, encrypted_value, created_at, updated_at, last_used_at, metadata_json
             FROM vault_items
             WHERE id = ?1",
            [secret_ref],
            |row| {
                let metadata_json: String = row.get(7)?;
                Ok(VaultItemView {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    label: row.get(2)?,
                    encrypted_value: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    last_used_at: row.get(6)?,
                    metadata: serde_json::from_str(&metadata_json).unwrap_or_else(|_| json!({})),
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub fn redact_secret(value: &str) -> String {
    let trimmed = value.trim();
    let suffix: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if suffix.is_empty() {
        "********".to_string()
    } else {
        format!("********{suffix}")
    }
}

fn encrypt_secret(db_path: &Path, plaintext: &[u8]) -> Result<String> {
    let key_bytes = load_vault_key(db_path)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| anyhow!("Failed to encrypt local appliance vault value."))?;
    Ok(format!(
        "{ENCRYPTED_VALUE_VERSION}:{}:{}",
        hex_encode(&nonce),
        hex_encode(&ciphertext)
    ))
}

fn decrypt_value(db_path: &Path, encrypted_value: &str) -> Result<Vec<u8>> {
    let parts = encrypted_value.split(':').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != ENCRYPTED_VALUE_VERSION {
        bail!("Unsupported local appliance vault value format.");
    }
    let nonce_bytes = hex_decode(parts[1])?;
    let ciphertext = hex_decode(parts[2])?;
    let key_bytes = load_vault_key(db_path)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|_| anyhow!("Failed to decrypt local appliance vault value."))
}

fn load_vault_key(db_path: &Path) -> Result<Vec<u8>> {
    let path = ensure_vault_key(db_path)?;
    read_vault_key(&path)
}

fn read_vault_key(path: &Path) -> Result<Vec<u8>> {
    let value = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read local appliance vault key at {}",
            path.display()
        )
    })?;
    let key = hex_decode(value.trim())?;
    if key.len() != VAULT_KEY_BYTES {
        bail!("Local appliance vault key is malformed.");
    }
    Ok(key)
}

fn restrict_key_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>> {
    let bytes = value.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        bail!("Hex value has invalid length.");
    }
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

fn hex_nibble(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("Hex value contains invalid character."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::init_schema;

    #[test]
    fn vault_key_creation_is_idempotent() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");

        let first = ensure_vault_key(&db_path).unwrap();
        let first_key = fs::read_to_string(&first).unwrap();
        let second = ensure_vault_key(&db_path).unwrap();
        let second_key = fs::read_to_string(&second).unwrap();

        assert_eq!(first, second);
        assert_eq!(first_key, second_key);
        assert_eq!(hex_decode(first_key.trim()).unwrap().len(), VAULT_KEY_BYTES);
    }

    #[test]
    fn vault_encrypts_and_decrypts_secret_without_plaintext_storage() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("local.db");
        let connection = Connection::open(&db_path).unwrap();
        init_schema(&connection).unwrap();

        let stored = store_secret(
            &db_path,
            &connection,
            "provider_api_key",
            "Anthropic API key",
            "sk-vault-test-secret",
            None,
            json!({ "providerId": "anthropic" }),
        )
        .unwrap();
        let item = get_vault_item(&connection, &stored.id).unwrap().unwrap();

        assert_ne!(item.encrypted_value, "sk-vault-test-secret");
        assert!(!item.encrypted_value.contains("sk-vault-test-secret"));
        assert_eq!(
            decrypt_secret(&db_path, &connection, &stored.id).unwrap(),
            "sk-vault-test-secret"
        );
    }
}
