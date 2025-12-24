#[cfg(feature = "encryption")]
use age::{Decryptor, Encryptor};
#[cfg(test)]
use fake::Dummy;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum EncryptionError {
  #[error("Encryption feature not enabled")]
  #[allow(unused)]
  NotEnabled,

  #[cfg(feature = "encryption")]
  #[error("Failed to encrypt file: {0}")]
  EncryptionFailed(#[from] age::EncryptError),

  #[cfg(feature = "encryption")]
  #[error("Failed to decrypt file: {0}")]
  DecryptionFailed(#[from] age::DecryptError),

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("No encryption key found")]
  NoKey,

  #[error("Expected encrypted file but found plaintext content")]
  UnexpectedPlaintext,
}

/// Configuration for encryption operations
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(test, derive(Dummy))]
pub struct EncryptionConfig {
  /// Path to the age identity file (private key)
  pub identity_path: Option<std::path::PathBuf>,
  /// Age recipient public key
  pub recipient: Option<String>,
}

impl Default for EncryptionConfig {
  fn default() -> Self {
    Self { identity_path: None, recipient: None }
  }
}

/// Encrypts data using age encryption
#[cfg(feature = "encryption")]
#[allow(unused)]
pub fn encrypt_data(data: &[u8], config: &EncryptionConfig) -> Result<Vec<u8>, EncryptionError> {
  use std::io::Write;

  let recipient = config.recipient.as_ref().ok_or(EncryptionError::NoKey)?;
  let recipient: age::x25519::Recipient = recipient.parse().map_err(|_| EncryptionError::NoKey)?;

  let recipients: Vec<&dyn age::Recipient> = vec![&recipient];
  let encryptor = Encryptor::with_recipients(recipients.into_iter())?;

  let mut encrypted = Vec::new();
  let mut writer = encryptor.wrap_output(&mut encrypted)?;

  writer.write_all(data)?;
  writer.finish()?;

  Ok(encrypted)
}

/// Decrypts data using age encryption
#[cfg(feature = "encryption")]
pub fn decrypt_data(encrypted_data: &[u8], config: &EncryptionConfig) -> Result<Vec<u8>, EncryptionError> {
  use std::io::{Cursor, Read};

  let identity_path = config.identity_path.as_ref().ok_or(EncryptionError::NoKey)?;
  let identity_data = std::fs::read_to_string(identity_path)?;
  let identity: age::x25519::Identity = identity_data.trim().parse().map_err(|_| EncryptionError::NoKey)?;

  let decryptor = Decryptor::new(Cursor::new(encrypted_data))?;

  let mut decrypted = Vec::new();
  let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?;

  reader.read_to_end(&mut decrypted)?;

  Ok(decrypted)
}

/// Stub functions for when encryption is disabled
#[cfg(not(feature = "encryption"))]
#[allow(unused)]
pub fn encrypt_data(_data: &[u8], _config: &EncryptionConfig) -> Result<Vec<u8>, EncryptionError> {
  Err(EncryptionError::NotEnabled)
}

#[cfg(not(feature = "encryption"))]
pub fn decrypt_data(_encrypted_data: &[u8], _config: &EncryptionConfig) -> Result<Vec<u8>, EncryptionError> {
  Err(EncryptionError::NotEnabled)
}

/// Determines synchronization direction based on file existence and link type
pub fn determine_sync_direction(source_exists: bool, target_exists: bool, link_type: &crate::config::LinkType) -> SyncDirection {
  match (source_exists, target_exists) {
    // If neither exists, create both empty
    (false, false) => SyncDirection::CreateBoth,
    // If only target exists, sync target -> source
    (false, true) => SyncDirection::TargetToSource,
    // If only source exists, sync source -> target
    (true, false) => SyncDirection::SourceToTarget,
    // If both exist, use link type to decide
    (true, true) => match link_type {
      crate::config::LinkType::Symbolic | crate::config::LinkType::Hard | crate::config::LinkType::Copy => SyncDirection::SourceToTarget,
      crate::config::LinkType::Record | crate::config::LinkType::Encrypted => SyncDirection::TargetToSource,
    },
  }
}

/// Direction for file synchronization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
  /// Create both files as empty
  CreateBoth,
  /// Synchronize from source to target
  SourceToTarget,
  /// Synchronize from target to source
  TargetToSource,
}

/// Enhanced link configuration that includes link type override
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(test, derive(Dummy, PartialEq, Eq))]
pub struct LinkConfig {
  /// Target paths for the link
  pub targets: std::collections::HashSet<std::path::PathBuf>,
  /// Link type override (overrides global link_type from config)
  #[serde(rename = "type")]
  pub link_type: Option<crate::config::LinkType>,
}

impl Default for LinkConfig {
  fn default() -> Self {
    Self {
      targets: std::collections::HashSet::new(),
      link_type: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sync_direction_determination() {
    use crate::config::LinkType;

    // Test all combinations with Copy (source -> target)
    assert_eq!(determine_sync_direction(false, false, &LinkType::Copy), SyncDirection::CreateBoth);
    assert_eq!(determine_sync_direction(false, true, &LinkType::Copy), SyncDirection::TargetToSource);
    assert_eq!(determine_sync_direction(true, false, &LinkType::Copy), SyncDirection::SourceToTarget);
    assert_eq!(determine_sync_direction(true, true, &LinkType::Copy), SyncDirection::SourceToTarget);

    // Test with Encrypted (target -> source)
    assert_eq!(determine_sync_direction(true, true, &LinkType::Encrypted), SyncDirection::TargetToSource);

    // Test with Record (target -> source)
    assert_eq!(determine_sync_direction(true, true, &LinkType::Record), SyncDirection::TargetToSource);
  }

  #[test]
  fn test_link_type_serialization() {
    use crate::config::LinkType;

    assert_eq!(serde_json::to_string(&LinkType::Copy).unwrap(), "\"Copy\"");
    assert_eq!(serde_json::to_string(&LinkType::Encrypted).unwrap(), "\"Encrypted\"");
    assert_eq!(serde_json::to_string(&LinkType::Record).unwrap(), "\"Record\"");
  }

  #[test]
  fn test_link_config_with_type_override() {
    use crate::config::LinkType;
    use std::path::PathBuf;

    let config = LinkConfig {
      targets: vec![PathBuf::from("/home/user/.test")].into_iter().collect(),
      link_type: Some(LinkType::Record),
    };

    assert_eq!(config.link_type, Some(LinkType::Record));
  }

  #[cfg(not(feature = "encryption"))]
  #[test]
  fn test_encryption_disabled() {
    let config = EncryptionConfig::default();
    assert!(encrypt_data(b"test", &config).is_err());
    assert!(decrypt_data(b"test", &config).is_err());
  }
}
