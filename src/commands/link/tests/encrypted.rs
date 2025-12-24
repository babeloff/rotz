//! Encryption-related tests for the rotz dotfile manager.
//!
//! This module contains comprehensive tests for encrypted file handling functionality,
//! including the three main encryption functions and their integration with the link system.
//!
//! # Test Categories
//!
//! ## Core Function Tests
//! - `encryptfile()` - Basic file/directory copying for encrypted content structure
//! - `sync_encrypted_files()` - Encryption/decryption logic with fallback behavior
//! - `is_likely_encrypted()` - Content detection heuristics for encrypted data
//!
//! ## Integration Tests
//! - Enhanced link creation with encrypted types
//! - Error handling for plaintext sources with encrypted link types
//! - Fallback behavior when encryption is disabled or keys unavailable
//!
//! ## Validation Tests
//! - Documentation example verification
//! - Comprehensive edge case coverage
//! - Configuration integration testing

use crate::config::LinkType;
use crate::encryption::{EncryptionConfig, LinkConfig};
use std::fs;
use tempfile::TempDir;
use velcro::hash_set;

// Import the necessary functions for testing
use super::super::{create_enhanced_link, create_link, encryptfile, is_likely_encrypted, sync_encrypted_files};

// ===== ENCRYPTION FUNCTIONALITY TESTS =====

#[cfg(feature = "encryption")]
#[test]
fn test_encrypted_file_management() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  // Create files with encrypted-looking content for encrypted type
  let encrypted_source = b"-----BEGIN AGE ENCRYPTED FILE-----\nencrypted source data\n-----END AGE ENCRYPTED FILE-----";
  let encrypted_target = b"-----BEGIN AGE ENCRYPTED FILE-----\nencrypted target data\n-----END AGE ENCRYPTED FILE-----";
  fs::write(&src_file, encrypted_source).unwrap();
  fs::write(&dst_file, encrypted_target).unwrap();

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Encrypted),
  };

  let encryption_config = EncryptionConfig::default(); // No keys configured
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  // Should succeed for encrypted-looking files (no actual encryption attempted)
  assert!(result.is_ok());
  // Record/Encrypted types sync target -> source when both exist
  assert_eq!(fs::read(&src_file).unwrap(), encrypted_target);
}

#[test]
fn test_encryptfile_basic_functionality() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  // Test basic file copying with encryptfile function
  fs::write(&src_file, "test content").unwrap();

  let result = encryptfile(&src_file, &dst_file);

  assert!(result.is_ok());
  assert!(dst_file.exists());
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "test content");
}

#[test]
fn test_encryptfile_creates_parent_directories() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("nested").join("dir").join("dest.txt");

  fs::write(&src_file, "test content").unwrap();

  let result = encryptfile(&src_file, &dst_file);

  assert!(result.is_ok());
  assert!(dst_file.exists());
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "test content");
}

#[test]
fn test_encryptfile_directory_handling() {
  let temp_dir = TempDir::new().unwrap();
  let src_dir = temp_dir.path().join("source_dir");
  let dst_dir = temp_dir.path().join("dest_dir");

  // Create source directory with a file
  fs::create_dir(&src_dir).unwrap();
  fs::write(src_dir.join("file.txt"), "directory content").unwrap();

  let result = encryptfile(&src_dir, &dst_dir);

  assert!(result.is_ok());
  assert!(dst_dir.exists());
  assert!(dst_dir.is_dir());
  assert_eq!(fs::read_to_string(dst_dir.join("file.txt")).unwrap(), "directory content");
}

#[test]
fn test_is_likely_encrypted() {
  // Test plaintext detection
  let plaintext = b"This is plain text content";
  assert!(!is_likely_encrypted(plaintext));

  // Test age encrypted file detection
  let age_encrypted = b"-----BEGIN AGE ENCRYPTED FILE-----\nsome encrypted data here";
  assert!(is_likely_encrypted(age_encrypted));

  // Test binary/encrypted-like content detection
  let binary_data = &[0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD, 0xFC];
  assert!(is_likely_encrypted(binary_data));

  // Test mixed content (mostly printable)
  let mixed_content = b"Hello world!\x00\x01\x02";
  assert!(!is_likely_encrypted(mixed_content));

  // Test empty data
  let empty_data = b"";
  assert!(!is_likely_encrypted(empty_data));
}

#[cfg(feature = "encryption")]
#[test]
fn test_sync_encrypted_files_behavior() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  // Test 1: Plaintext source should return error
  fs::write(&src_file, "plaintext content").unwrap();

  let encryption_config = EncryptionConfig::default(); // No keys configured
  let result = sync_encrypted_files(&src_file, &dst_file, &encryption_config);

  // Should fail with UnexpectedPlaintext error
  assert!(result.is_err());
  match result {
    Err(super::super::Error::Encryption(crate::encryption::EncryptionError::UnexpectedPlaintext)) => {
      // This is expected
    }
    _ => panic!("Expected UnexpectedPlaintext error, got: {:?}", result),
  }

  // Test 2: Encrypted-looking source should succeed (fallback to copying)
  let encrypted_like_content = b"-----BEGIN AGE ENCRYPTED FILE-----\nsome encrypted data here";
  fs::write(&src_file, encrypted_like_content).unwrap();

  let result = sync_encrypted_files(&src_file, &dst_file, &encryption_config);

  // Should succeed even without encryption keys (falls back to copying)
  assert!(result.is_ok());
  assert!(dst_file.exists());
  // When encryption is not enabled, should copy as-is
  assert_eq!(fs::read(&dst_file).unwrap(), encrypted_like_content);
}

/// Test to validate that the documented behavior examples are accurate
#[test]
fn test_documentation_examples() {
  // Test encryptfile behavior as documented
  let temp_dir = TempDir::new().unwrap();
  let source = temp_dir.path().join("source.txt");
  let dest = temp_dir.path().join("dest.txt");

  fs::write(&source, "test content").unwrap();
  let result = encryptfile(&source, &dest);

  assert!(result.is_ok());
  assert!(dest.exists());
  assert_eq!(fs::read_to_string(&dest).unwrap(), "test content");

  // Test directory copying as documented
  let source_dir = temp_dir.path().join("source_dir");
  let dest_dir = temp_dir.path().join("dest_dir");

  fs::create_dir(&source_dir).unwrap();
  fs::write(source_dir.join("file.txt"), "directory content").unwrap();

  let result = encryptfile(&source_dir, &dest_dir);
  assert!(result.is_ok());
  assert!(dest_dir.is_dir());
  assert_eq!(fs::read_to_string(dest_dir.join("file.txt")).unwrap(), "directory content");

  // Test is_likely_encrypted detection as documented
  let age_data = b"-----BEGIN AGE ENCRYPTED FILE-----\nabc123...";
  assert!(is_likely_encrypted(age_data));

  let plain_text = b"Hello, this is plain text content!";
  assert!(!is_likely_encrypted(plain_text));

  let binary_data = &[0xFF, 0x00, 0xAB, 0xCD, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
  assert!(is_likely_encrypted(binary_data));

  assert!(!is_likely_encrypted(b""));

  let text_with_whitespace = b"Hello\nWorld\t!";
  assert!(!is_likely_encrypted(text_with_whitespace));
}

#[test]
fn test_create_link_with_encrypted_type() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  let encrypted_content = b"-----BEGIN AGE ENCRYPTED FILE-----\nencrypted integration test data\n-----END AGE ENCRYPTED FILE-----";
  fs::write(&src_file, encrypted_content).unwrap();

  let result = create_link(&src_file, &dst_file, &LinkType::Encrypted, false, None);
  assert!(result.is_ok());

  assert!(dst_file.exists());
  // Encrypted type should copy the content (since no encryption keys configured)
  assert_eq!(fs::read(&dst_file).unwrap(), encrypted_content);
}

#[test]
fn test_create_enhanced_link_encrypted_with_plaintext_source() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  // Create plaintext source file
  fs::write(&src_file, "plaintext source content").unwrap();
  fs::write(&dst_file, "plaintext target content").unwrap();

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Encrypted),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  // Should fail because plaintext source is not expected for Encrypted type
  assert!(result.is_err());
  match result {
    Err(super::super::Error::Encryption(crate::encryption::EncryptionError::UnexpectedPlaintext)) => {
      // This is expected
    }
    _ => panic!("Expected UnexpectedPlaintext error, got: {:?}", result),
  }
}
