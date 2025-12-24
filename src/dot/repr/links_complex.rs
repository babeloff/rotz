use std::{collections::HashSet, path::PathBuf};

#[cfg(test)]
use fake::Dummy;
use serde::Deserialize;

use crate::encryption::LinkConfig;

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
#[cfg_attr(test, derive(Dummy))]
#[serde(deny_unknown_fields)]
pub(super) enum LinksComplex {
  /// Simple path specification (backward compatibility)
  One(PathBuf),
  /// Multiple paths (backward compatibility)
  Many(HashSet<PathBuf>),
  /// Enhanced configuration with file management type
  Enhanced(LinkConfig),
  /// Map of path to enhanced configuration
  EnhancedMap(std::collections::HashMap<PathBuf, EnhancedLinkTarget>),
}

#[derive(Deserialize, Clone, Debug)]
#[cfg_attr(test, derive(Dummy))]
#[serde(untagged)]
pub(super) enum EnhancedLinkTarget {
  /// Single target with link type override
  Single {
    target: PathBuf,
    #[serde(rename = "type")]
    link_type: Option<crate::config::LinkType>,
  },
  /// Multiple targets with shared link type override
  Multiple {
    targets: HashSet<PathBuf>,
    #[serde(rename = "type")]
    link_type: Option<crate::config::LinkType>,
  },
  /// Just the link type override (uses source path as target)
  TypeOnly {
    #[serde(rename = "type")]
    link_type: Option<crate::config::LinkType>,
  },
}
