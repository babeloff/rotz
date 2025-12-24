use crate::config::LinkType;
use crate::encryption::{EncryptionConfig, LinkConfig};
use std::fs;
use tempfile::TempDir;
use velcro::hash_set;

// Import the necessary functions for testing
use super::super::{copy_dir_all, copyfile, create_enhanced_link, create_link, encryptfile, is_likely_encrypted, sync_encrypted_files};

// ===== COPY FUNCTIONALITY TESTS =====

#[test]
fn test_copyfile_single_file() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&src_file, "test content").unwrap();

  copyfile(&src_file, &dst_file).unwrap();

  assert!(dst_file.exists());
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "test content");
}

#[test]
fn test_copyfile_directory() {
  let temp_dir = TempDir::new().unwrap();
  let src_dir = temp_dir.path().join("source_dir");
  let dst_dir = temp_dir.path().join("dest_dir");

  fs::create_dir(&src_dir).unwrap();
  fs::write(src_dir.join("file1.txt"), "content1").unwrap();
  fs::write(src_dir.join("file2.txt"), "content2").unwrap();

  copyfile(&src_dir, &dst_dir).unwrap();

  assert!(dst_dir.exists());
  assert!(dst_dir.join("file1.txt").exists());
  assert!(dst_dir.join("file2.txt").exists());
  assert_eq!(fs::read_to_string(dst_dir.join("file1.txt")).unwrap(), "content1");
  assert_eq!(fs::read_to_string(dst_dir.join("file2.txt")).unwrap(), "content2");
}

#[test]
fn test_copy_dir_all_nested() {
  let temp_dir = TempDir::new().unwrap();
  let src_dir = temp_dir.path().join("source");
  let dst_dir = temp_dir.path().join("dest");
  let nested_dir = src_dir.join("nested");

  fs::create_dir_all(&nested_dir).unwrap();
  fs::write(src_dir.join("root.txt"), "root content").unwrap();
  fs::write(nested_dir.join("nested.txt"), "nested content").unwrap();

  copy_dir_all(&src_dir, &dst_dir).unwrap();

  assert!(dst_dir.exists());
  assert!(dst_dir.join("root.txt").exists());
  assert!(dst_dir.join("nested").exists());
  assert!(dst_dir.join("nested/nested.txt").exists());
  assert_eq!(fs::read_to_string(dst_dir.join("root.txt")).unwrap(), "root content");
  assert_eq!(fs::read_to_string(dst_dir.join("nested/nested.txt")).unwrap(), "nested content");
}

#[test]
fn test_copyfile_creates_parent_directories() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("nested").join("deeply").join("nested").join("dest.txt");

  fs::write(&src_file, "test content").unwrap();

  copyfile(&src_file, &dst_file).unwrap();

  assert!(dst_file.exists());
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "test content");
}

#[test]
fn test_copyfile_empty_directory() {
  let temp_dir = TempDir::new().unwrap();
  let src_dir = temp_dir.path().join("empty_source");
  let dst_dir = temp_dir.path().join("empty_dest");

  fs::create_dir(&src_dir).unwrap();

  copyfile(&src_dir, &dst_dir).unwrap();

  assert!(dst_dir.exists());
  assert!(dst_dir.is_dir());
  assert_eq!(fs::read_dir(&dst_dir).unwrap().count(), 0);
}

#[test]
fn test_copy_dir_all_preserves_structure() {
  let temp_dir = TempDir::new().unwrap();
  let src_dir = temp_dir.path().join("complex_source");
  let dst_dir = temp_dir.path().join("complex_dest");

  // Create a complex directory structure
  let subdir1 = src_dir.join("subdir1");
  let subdir2 = src_dir.join("subdir2");
  let nested = subdir1.join("nested");

  fs::create_dir_all(&nested).unwrap();
  fs::create_dir_all(&subdir2).unwrap();

  fs::write(src_dir.join("root.txt"), "root").unwrap();
  fs::write(subdir1.join("sub1.txt"), "sub1").unwrap();
  fs::write(subdir2.join("sub2.txt"), "sub2").unwrap();
  fs::write(nested.join("nested.txt"), "nested").unwrap();

  copy_dir_all(&src_dir, &dst_dir).unwrap();

  // Verify structure is preserved
  assert!(dst_dir.join("root.txt").exists());
  assert!(dst_dir.join("subdir1").join("sub1.txt").exists());
  assert!(dst_dir.join("subdir2").join("sub2.txt").exists());
  assert!(dst_dir.join("subdir1").join("nested").join("nested.txt").exists());

  // Verify content is preserved
  assert_eq!(fs::read_to_string(dst_dir.join("root.txt")).unwrap(), "root");
  assert_eq!(fs::read_to_string(dst_dir.join("subdir1").join("sub1.txt")).unwrap(), "sub1");
  assert_eq!(fs::read_to_string(dst_dir.join("subdir2").join("sub2.txt")).unwrap(), "sub2");
  assert_eq!(fs::read_to_string(dst_dir.join("subdir1").join("nested").join("nested.txt")).unwrap(), "nested");
}

#[test]
fn test_create_link_with_copy_type() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&src_file, "integration test").unwrap();

  let result = create_link(&src_file, &dst_file, &LinkType::Copy, false, None);
  assert!(result.is_ok());

  assert!(dst_file.exists());
  assert!(!dst_file.is_symlink());
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "integration test");
}

#[test]
fn test_create_link_copy_overwrites_existing() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&src_file, "source content").unwrap();
  fs::write(&dst_file, "existing content").unwrap();

  // Copy type always overwrites existing files
  let result = create_link(&src_file, &dst_file, &LinkType::Copy, false, None);
  assert!(result.is_ok());
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "source content");
}

// ===== ENHANCED COPY FUNCTIONALITY TESTS =====

#[test]
fn test_create_enhanced_link_internal_both_exist() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&src_file, "source content").unwrap();
  fs::write(&dst_file, "target content").unwrap();

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Copy),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Internal type should sync source -> target
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "source content");
}

#[test]
fn test_create_enhanced_link_external_both_exist() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&src_file, "source content").unwrap();
  fs::write(&dst_file, "target content").unwrap();

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Record),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Record type should sync target -> source
  assert_eq!(fs::read_to_string(&src_file).unwrap(), "target content");
}

#[test]
fn test_create_enhanced_link_neither_exist() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Copy),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Both should be created as empty
  assert!(src_file.exists());
  assert!(dst_file.exists());
  assert_eq!(fs::read_to_string(&src_file).unwrap(), "");
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "");
}

#[test]
fn test_create_enhanced_link_only_source_exists() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&src_file, "source content").unwrap();

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Copy),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Should sync source -> target
  assert_eq!(fs::read_to_string(&dst_file).unwrap(), "source content");
}

#[test]
fn test_create_enhanced_link_only_target_exists() {
  let temp_dir = TempDir::new().unwrap();
  let src_file = temp_dir.path().join("source.txt");
  let dst_file = temp_dir.path().join("dest.txt");

  fs::write(&dst_file, "target content").unwrap();

  let link_config = LinkConfig {
    targets: hash_set![dst_file.clone()],
    link_type: Some(LinkType::Copy),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = link_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_file, &dst_file, effective_link_type, &link_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Should sync target -> source
  assert_eq!(fs::read_to_string(&src_file).unwrap(), "target content");
}

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

#[test]
fn test_enhanced_copy_integration() {
  let temp_dir = TempDir::new().unwrap();
  let dotfiles_dir = temp_dir.path().join("dotfiles");
  let home_dir = temp_dir.path().join("home");

  fs::create_dir_all(&dotfiles_dir).unwrap();
  fs::create_dir_all(&home_dir).unwrap();

  // Setup test files
  let src_internal = dotfiles_dir.join("internal.txt");
  let dst_internal = home_dir.join("internal.txt");
  let src_external = dotfiles_dir.join("external.txt");
  let dst_external = home_dir.join("external.txt");

  // Test internal management: source -> target when both exist
  fs::write(&src_internal, "source content").unwrap();
  fs::write(&dst_internal, "target content").unwrap();

  let internal_config = LinkConfig {
    targets: hash_set![dst_internal.clone()],
    link_type: Some(LinkType::Copy),
  };

  let encryption_config = EncryptionConfig::default();
  let effective_link_type = internal_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_internal, &dst_internal, effective_link_type, &internal_config, false, None, &encryption_config);

  assert!(result.is_ok());
  assert_eq!(fs::read_to_string(&dst_internal).unwrap(), "source content");

  // Test record management: target -> source when both exist
  fs::write(&src_external, "old source").unwrap();
  fs::write(&dst_external, "target external").unwrap();

  let record_config = LinkConfig {
    targets: hash_set![dst_external.clone()],
    link_type: Some(LinkType::Record),
  };

  let effective_link_type = record_config.link_type.as_ref().unwrap_or(&LinkType::Copy);
  let result = create_enhanced_link(&src_external, &dst_external, effective_link_type, &record_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Record type: target -> source when both exist
  assert_eq!(fs::read_to_string(&src_external).unwrap(), "target external");
}

#[test]
fn test_type_override_system_integration() {
  let temp_dir = TempDir::new().unwrap();
  let dotfiles_dir = temp_dir.path().join("dotfiles");
  let home_dir = temp_dir.path().join("home");

  fs::create_dir_all(&dotfiles_dir).unwrap();
  fs::create_dir_all(&home_dir).unwrap();

  // Test 1: No type override - should use global default (Copy behavior)
  let src_default = dotfiles_dir.join("default.txt");
  let dst_default = home_dir.join("default.txt");
  fs::write(&src_default, "source default").unwrap();
  fs::write(&dst_default, "target default").unwrap();

  let default_config = LinkConfig {
    targets: hash_set![dst_default.clone()],
    link_type: None, // No override - uses global default
  };

  let encryption_config = EncryptionConfig::default();
  let global_link_type = LinkType::Copy; // Simulate global config
  let effective_link_type = default_config.link_type.as_ref().unwrap_or(&global_link_type);

  let result = create_enhanced_link(&src_default, &dst_default, effective_link_type, &default_config, false, None, &encryption_config);
  assert!(result.is_ok());
  // Copy type: source -> target when both exist
  assert_eq!(fs::read_to_string(&dst_default).unwrap(), "source default");

  // Test 2: Explicit type override to Record
  let src_record = dotfiles_dir.join("record.txt");
  let dst_record = home_dir.join("record.txt");
  fs::write(&src_record, "source record").unwrap();
  fs::write(&dst_record, "target record").unwrap();

  let record_config = LinkConfig {
    targets: hash_set![dst_record.clone()],
    link_type: Some(LinkType::Record), // Override to Record
  };

  let effective_link_type = record_config.link_type.as_ref().unwrap_or(&global_link_type);
  let result = create_enhanced_link(&src_record, &dst_record, effective_link_type, &record_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Record type: target -> source when both exist
  assert_eq!(fs::read_to_string(&src_record).unwrap(), "target record");

  // Test 3: Override to Copy (same as global but explicit)
  let src_copy = dotfiles_dir.join("copy.txt");
  let dst_copy = home_dir.join("copy.txt");
  fs::write(&src_copy, "source copy").unwrap();
  fs::write(&dst_copy, "target copy").unwrap();

  let copy_config = LinkConfig {
    targets: hash_set![dst_copy.clone()],
    link_type: Some(LinkType::Copy), // Explicit override to Copy
  };

  let effective_link_type = copy_config.link_type.as_ref().unwrap_or(&global_link_type);
  let result = create_enhanced_link(&src_copy, &dst_copy, effective_link_type, &copy_config, false, None, &encryption_config);

  assert!(result.is_ok());
  // Copy type: source -> target when both exist
  assert_eq!(fs::read_to_string(&dst_copy).unwrap(), "source copy");

  // Test 4: Multiple targets with type override
  let src_multi = dotfiles_dir.join("multi.txt");
  let dst_multi1 = home_dir.join("multi1.txt");
  let dst_multi2 = home_dir.join("multi2.txt");

  fs::write(&src_multi, "source multi").unwrap();

  let multi_config = LinkConfig {
    targets: hash_set![dst_multi1.clone(), dst_multi2.clone()],
    link_type: Some(LinkType::Copy),
  };

  let effective_link_type = multi_config.link_type.as_ref().unwrap_or(&global_link_type);
  for target in &multi_config.targets {
    let individual_config = LinkConfig {
      targets: hash_set![target.clone()],
      link_type: multi_config.link_type.clone(),
    };
    let result = create_enhanced_link(&src_multi, target, effective_link_type, &individual_config, false, None, &encryption_config);
    assert!(result.is_ok());
    assert_eq!(fs::read_to_string(target).unwrap(), "source multi");
  }
}
