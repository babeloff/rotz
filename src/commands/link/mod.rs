use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
  fs,
  path::{Path, PathBuf},
};

use crossterm::style::{Attribute, Stylize};
use itertools::Itertools;
use miette::{Diagnostic, Report, Result};
use tap::Pipe;
#[cfg(feature = "profiling")]
use tracing::instrument;
use velcro::hash_map;
use wax::Pattern;

use super::Command;
use crate::{
  config::{Config, LinkType},
  encryption::{EncryptionConfig, EncryptionError, LinkConfig, SyncDirection, determine_sync_direction},
  helpers,
  state::{self},
  templating,
};

#[derive(thiserror::Error, Diagnostic, Debug)]
pub(crate) enum Error {
  #[error("Could not create link from \"{0}\" to \"{1}\"")]
  #[cfg_attr(windows, diagnostic(code(link::linking), help("You may need to run Rotz from an admin shell to create file links")))]
  #[cfg_attr(not(windows), diagnostic(code(link::linking),))]
  Symlink(PathBuf, PathBuf, #[source] std::io::Error),

  #[error("Could not remove orphaned link from \"{0}\" to \"{1}\"")]
  #[diagnostic(code(link::orphan::remove))]
  RemovingOrphan(PathBuf, PathBuf, #[source] std::io::Error),

  #[error("The file \"{0}\" already exists")]
  #[diagnostic(code(link::already_exists), help("Try using the --force flag"))]
  AlreadyExists(PathBuf),

  #[error("The link source file \"{0}\" does not exist exists")]
  #[diagnostic(code(link::does_not_exist), help("Maybe you have a typo in the filename?"))]
  LinkSourceDoesNotExist(PathBuf),

  #[error("Encryption error: {0}")]
  #[diagnostic(code(link::encryption))]
  Encryption(#[from] EncryptionError),

  #[error("IO error: {0}")]
  #[diagnostic(code(link::io))]
  Io(#[from] std::io::Error),
}

pub(crate) struct Link<'a> {
  config: Config,
  engine: templating::Engine<'a>,
}

impl Debug for Link<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Link").field("config", &self.config).finish()
  }
}

impl<'a> Link<'a> {
  pub const fn new(config: crate::config::Config, engine: templating::Engine<'a>) -> Self {
    Self { config, engine }
  }
}

impl<'a> Command for Link<'a> {
  type Args = (crate::cli::Globals, crate::cli::Link, &'a state::Linked);
  type Result = Result<state::Linked>;

  #[cfg_attr(feature = "profiling", instrument)]
  fn execute(&self, (globals, link_command, linked): Self::Args) -> Self::Result {
    let links = crate::dot::read_dots(&self.config.dotfiles, &link_command.dots, &self.config, &self.engine)?
      .into_iter()
      .filter_map(|d| d.1.links.map(|l| (d.0, l)))
      .collect_vec();

    {
      let current_links = links
        .iter()
        .flat_map(|l| l.1.iter().map(|h| h.1.targets.iter()))
        .flatten()
        .map(helpers::resolve_home)
        .collect::<HashSet<_>>();

      let mut errors = Vec::new();

      let dots = helpers::glob_from_vec(&link_command.dots, None)?;
      let linked = linked.0.iter().filter(|l| dots.is_match(l.0.as_str()));

      for (name, links) in linked {
        let mut printed = false;
        for (to, from) in links {
          if !current_links.contains(to) {
            let mut removed = true;
            if !globals.dry_run {
              if let Err(err) = fs::remove_file(to) {
                removed = false;

                if err.kind() != std::io::ErrorKind::NotFound {
                  errors.push(Error::RemovingOrphan(from.clone(), to.clone(), err));
                }
              }
            }

            if removed {
              if !printed {
                println!("{}Removing orphans for {}{}\n", Attribute::Bold, name.as_str().dark_blue(), Attribute::Reset);
                printed = true;
              }
              println!("  x {}", to.to_string_lossy().dark_green());
            }
          }
        }

        if printed {
          println!();
        }
      }

      helpers::join_err(errors)?;
    }

    let mut new_linked = hash_map!();

    for (name, link) in links {
      println!("{}Linking {}{}\n", Attribute::Bold, name.as_str().dark_blue(), Attribute::Reset);

      let mut new_linked_inner = hash_map!();

      let base_path = self.config.dotfiles.join(&name[1..]);
      for (from, link_config) in link {
        for to in &link_config.targets {
          let effective_link_type = link_config.link_type.as_ref().unwrap_or(&self.config.link_type);
          println!("  {} -> {} ({})", from.to_string_lossy().dark_green(), to.to_string_lossy().dark_green(), effective_link_type);
          let from_path = base_path.join(&from);
          let to_path = helpers::resolve_home(&to);

          if !globals.dry_run {
            if let Err(err) = create_enhanced_link(
              &from_path,
              &to_path,
              effective_link_type,
              &link_config,
              link_command.force,
              linked.0.get(&name),
              &self.config.encryption,
            ) {
              eprintln!("\n Error: {:?}", Report::new(err));
            } else {
              new_linked_inner.insert(to_path.clone(), from_path.clone());
            }
          }
        }
      }

      if !new_linked_inner.is_empty() {
        new_linked.insert(name, new_linked_inner);
      }

      println!();
    }

    state::Linked(new_linked).pipe(Ok)
  }
}

#[cfg_attr(feature = "profiling", instrument)]
fn create_link(from: &Path, to: &Path, link_type: &LinkType, force: bool, linked: Option<&HashMap<PathBuf, PathBuf>>) -> std::result::Result<(), Error> {
  if !from.exists() {
    return Error::LinkSourceDoesNotExist(from.to_path_buf()).pipe(Err);
  }

  let create: fn(&Path, &Path) -> std::result::Result<(), std::io::Error> = match link_type {
    LinkType::Symbolic => symlink,
    LinkType::Hard => hardlink,
    LinkType::Copy | LinkType::Record => copyfile,
    LinkType::Encrypted => encryptfile,
  };

  match create(from, to) {
    Ok(ok) => ok.pipe(Ok),
    Err(err) => match err.kind() {
      std::io::ErrorKind::AlreadyExists => {
        if force || linked.is_some_and(|l| l.contains_key(to)) {
          if to.is_dir() { fs::remove_dir_all(to) } else { fs::remove_file(to) }.map_err(|e| Error::Symlink(from.to_path_buf(), to.to_path_buf(), e))?;
          create(from, to)
        } else {
          return Error::AlreadyExists(to.to_path_buf()).pipe(Err);
        }
      }
      _ => err.pipe(Err),
    },
  }
  .map_err(|e| Error::Symlink(from.to_path_buf(), to.to_path_buf(), e))
}

#[cfg(windows)]
#[cfg_attr(feature = "profiling", instrument)]
pub(crate) fn symlink(from: &Path, to: &Path) -> std::io::Result<()> {
  use std::os::windows::fs;

  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if from.is_dir() {
    fs::symlink_dir(from, to)?;
  } else {
    fs::symlink_file(from, to)?;
  };
  ().pipe(Ok)
}

#[cfg(unix)]
#[cfg_attr(feature = "profiling", instrument)]
pub(crate) fn symlink(from: &Path, to: &Path) -> std::io::Result<()> {
  use std::os::unix::fs;
  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }
  fs::symlink(from, to)?;
  ().pipe(Ok)
}

#[cfg(windows)]
#[cfg_attr(feature = "profiling", instrument)]
pub(crate) fn hardlink(from: &Path, to: &Path) -> std::io::Result<()> {
  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if from.is_dir() {
    junction::create(from, to)?;
  } else {
    fs::hard_link(from, to)?;
  }
  ().pipe(Ok)
}

#[cfg(unix)]
#[cfg_attr(feature = "profiling", instrument)]
pub(crate) fn hardlink(from: &Path, to: &Path) -> std::io::Result<()> {
  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }
  fs::hard_link(from, to)?;
  ().pipe(Ok)
}

#[cfg_attr(feature = "profiling", instrument)]
pub(crate) fn copyfile(from: &Path, to: &Path) -> std::io::Result<()> {
  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if from.is_dir() {
    copy_dir_all(from, to)?;
  } else {
    fs::copy(from, to)?;
  }
  ().pipe(Ok)
}

#[cfg_attr(feature = "profiling", instrument)]
fn create_enhanced_link(
  from: &Path,
  to: &Path,
  link_type: &LinkType,
  _link_config: &LinkConfig,
  force: bool,
  linked: Option<&HashMap<PathBuf, PathBuf>>,
  encryption_config: &EncryptionConfig,
) -> std::result::Result<(), Error> {
  let source_exists = from.exists();
  let target_exists = to.exists();

  // Determine synchronization direction
  let sync_direction = determine_sync_direction(source_exists, target_exists, link_type);

  match sync_direction {
    SyncDirection::CreateBoth => {
      // Create both files as empty
      if let Some(parent) = from.parent() {
        std::fs::create_dir_all(parent)?;
      }
      if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
      }
      fs::write(from, "")?;
      fs::write(to, "")?;
    }
    SyncDirection::SourceToTarget => {
      sync_files(from, to, link_type, encryption_config)?;
    }
    SyncDirection::TargetToSource => {
      sync_files(to, from, link_type, encryption_config)?;
    }
  }

  // After synchronization, create the actual link
  match link_type {
    LinkType::Copy | LinkType::Record | LinkType::Encrypted => {
      // For copy-based types, the sync_files already handled the copying
      Ok(())
    }
    LinkType::Symbolic | LinkType::Hard => {
      // Create the actual link (symbolic or hard)
      create_link(from, to, link_type, force, linked)
    }
  }
}

/// Creates a file or directory for encrypted content.
///
/// Copies files or directories for the `Encrypted` link type with basic structure creation.
///
/// This function is specifically designed for the `Encrypted` link type and handles the basic
/// file/directory structure creation. The actual encryption/decryption logic is handled
/// by `sync_files` in the enhanced link creation process.
///
/// # Arguments
///
/// * `from` - Source path to copy from
/// * `to` - Destination path to copy to
///
/// # Returns
///
/// Returns `Ok(())` on success, or an `std::io::Error` if the copy operation fails.
///
/// # Behavior
///
/// - Creates parent directories for the destination if they don't exist
/// - Copies directories recursively using `copy_dir_all`
/// - Copies individual files directly without modification
/// - Does not perform actual encryption/decryption (handled elsewhere)
///
#[cfg_attr(feature = "profiling", instrument)]
pub(crate) fn encryptfile(from: &Path, to: &Path) -> std::result::Result<(), std::io::Error> {
  // For encrypted files, the actual encryption/decryption logic is handled
  // by sync_files in create_enhanced_link, so this just ensures the basic
  // file structure is in place
  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if from.is_dir() {
    copy_dir_all(from, to)?;
  } else {
    // Just copy the file - encryption logic is handled in sync_files
    std::fs::copy(from, to)?;
  }

  Ok(())
}

fn sync_files(source: &Path, target: &Path, link_type: &LinkType, encryption_config: &EncryptionConfig) -> std::result::Result<(), Error> {
  if let Some(parent) = target.parent() {
    std::fs::create_dir_all(parent)?;
  }

  if source.is_dir() {
    copy_dir_all(source, target)?;
    return Ok(());
  }

  match link_type {
    LinkType::Encrypted => {
      sync_encrypted_files(source, target, encryption_config)?;
    }
    _ => {
      // For copy and record types, just copy
      fs::copy(source, target)?;
    }
  }

  Ok(())
}

/// Synchronizes encrypted files with proper decryption handling.
///
/// This function handles the core encryption logic for the `Encrypted` link type.
/// It expects the source file to be encrypted and attempts to decrypt it to the target location.
/// If encryption is disabled or keys are missing, it falls back to plain copying.
///
/// # Arguments
///
/// * `source` - Path to the encrypted source file
/// * `target` - Path where the decrypted content should be written
/// * `encryption_config` - Configuration containing encryption keys and settings
///
/// # Returns
///
/// Returns `Ok(())` on success, or an `Error` if:
/// - The source file appears to be plaintext (returns `UnexpectedPlaintext` error)
/// - File I/O operations fail
/// - Decryption fails with an unrecoverable error
///
/// # Behavior
///
/// 1. Reads the source file data
/// 2. Validates that the source appears to be encrypted using `is_likely_encrypted`
/// 3. Attempts to decrypt the data using the provided encryption configuration
/// 4. Falls back to copying as-is if encryption is disabled or no key is available
/// 5. Writes the result (decrypted or original) to the target path
///
/// # Errors
///
/// This function returns `Error::Encryption(EncryptionError::UnexpectedPlaintext)`
/// if the source file appears to contain plaintext content rather than encrypted data.
//
pub(crate) fn sync_encrypted_files(source: &Path, target: &Path, encryption_config: &EncryptionConfig) -> std::result::Result<(), Error> {
  use crate::encryption::{EncryptionError, decrypt_data};

  let source_data = fs::read(source)?;

  // For encrypted files, we expect the source to be encrypted
  // If it's not encrypted, report an error
  if !is_likely_encrypted(&source_data) {
    return Err(Error::Encryption(EncryptionError::UnexpectedPlaintext));
  }

  // Source is encrypted, decrypt to target
  let target_data = match decrypt_data(&source_data, encryption_config) {
    Ok(decrypted) => decrypted,
    Err(crate::encryption::EncryptionError::NotEnabled) | Err(crate::encryption::EncryptionError::NoKey) => {
      // If encryption is not enabled or no key is configured, just copy as-is
      source_data
    }
    Err(e) => return Err(Error::Encryption(e)),
  };

  fs::write(target, target_data)?;
  Ok(())
}

/// Determines if file data is likely encrypted based on content heuristics.
///
/// This function uses simple heuristics to detect encrypted content by examining
/// the data for common patterns found in encrypted files.
///
/// # Arguments
///
/// * `data` - Byte slice containing the file data to analyze
///
/// # Returns
///
/// Returns `true` if the data appears to be encrypted or binary, `false` if it appears to be plaintext.
///
/// # Detection Methods
///
/// 1. **Age Encryption Header**: Checks if the data starts with the standard age encryption header
/// 2. **Non-printable Character Ratio**: Counts non-printable characters (excluding whitespace)
///    and considers the data encrypted if more than 20% of bytes are non-printable
///
/// # Limitations
///
/// This is a heuristic approach and may produce false positives or negatives:
/// - Binary files (images, executables) may be detected as "encrypted"
/// - Sophisticated plaintext that resembles encrypted data might be misclassified
/// - Very short files may not provide enough data for accurate detection
pub(crate) fn is_likely_encrypted(data: &[u8]) -> bool {
  // Simple heuristic: if the data starts with age encryption header
  // or contains a high proportion of non-printable characters, consider it encrypted
  if data.starts_with(b"-----BEGIN AGE ENCRYPTED FILE-----") {
    return true;
  }

  // Count non-printable characters (excluding common whitespace)
  let non_printable_count = data.iter().filter(|&&b| !b.is_ascii_graphic() && !matches!(b, b' ' | b'\t' | b'\n' | b'\r')).count();

  // If more than 20% of bytes are non-printable, likely encrypted/binary
  data.len() > 0 && (non_printable_count as f64 / data.len() as f64) > 0.2
}

pub(crate) fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
  fs::create_dir_all(dst)?;
  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let ty = entry.file_type()?;
    if ty.is_dir() {
      copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
    } else {
      fs::copy(&entry.path(), &dst.join(entry.file_name()))?;
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  mod common;
  mod copy;
  mod encrypted;
  mod hard;
  mod symbolic;
}
