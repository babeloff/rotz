use std::{
  collections::{HashMap, HashSet},
  path::PathBuf,
};

#[cfg(test)]
use fake::Dummy;
use itertools::{Either, Itertools};
use serde::Deserialize;
use tap::Pipe;
#[cfg(feature = "profiling")]
use tracing::instrument;
use velcro::hash_set;

use crate::{
  encryption::LinkConfig,
  helpers::{self, MultipleErrors},
  templating::{Engine, Parameters},
};

use super::{CapabilitiesComplex, DotCanonical, EnhancedLinkTarget, InstallsCanonical, LinksComplex, Merge};

#[derive(Deserialize, Clone, Default, Debug)]
#[cfg_attr(test, derive(Dummy))]
#[serde(deny_unknown_fields)]
pub struct CapabilitiesCanonical {
  pub links: Option<HashMap<PathBuf, LinkConfig>>,
  pub installs: Option<InstallsCanonical>,
  pub depends: Option<HashSet<String>>,
}

impl From<CapabilitiesComplex> for CapabilitiesCanonical {
  #[cfg_attr(feature = "profiling", instrument)]
  fn from(value: CapabilitiesComplex) -> Self {
    Self {
      links: value.links.map(|links| {
        links
          .into_iter()
          .map(|(source_path, link_complex)| {
            let link_config = match link_complex {
              LinksComplex::One(target) => LinkConfig {
                targets: hash_set!(target),
                link_type: None,
              },
              LinksComplex::Many(targets) => LinkConfig { targets, link_type: None },
              LinksComplex::Enhanced(config) => config,
              LinksComplex::EnhancedMap(map) => {
                // For enhanced map, we need to flatten it into individual configs
                // This is a simplified approach - in practice, you might want to handle this differently
                let mut all_targets = HashSet::new();
                let mut link_type = None;

                for (target_path, enhanced_target) in map {
                  match enhanced_target {
                    EnhancedLinkTarget::Single { target, link_type: lt } => {
                      all_targets.insert(target);
                      link_type = lt;
                    }
                    EnhancedLinkTarget::Multiple { targets, link_type: lt } => {
                      all_targets.extend(targets);
                      link_type = lt;
                    }
                    EnhancedLinkTarget::TypeOnly { link_type: lt } => {
                      all_targets.insert(target_path);
                      link_type = lt;
                    }
                  }
                }

                LinkConfig { targets: all_targets, link_type }
              }
            };
            (source_path, link_config)
          })
          .collect::<HashMap<_, _>>()
      }),
      installs: value.installs.map(Into::into),
      depends: value.depends,
    }
  }
}

impl CapabilitiesCanonical {
  #[cfg_attr(feature = "profiling", instrument(skip(engine)))]
  pub fn from(DotCanonical { selectors }: DotCanonical, engine: &Engine<'_>, parameters: &Parameters<'_>) -> Result<Self, helpers::ParseError> {
    let selectors = selectors
      .into_iter()
      .map(|(selector, capabilities)| (selector.applies(engine, parameters), selector, capabilities))
      .collect_vec();
    if selectors.iter().any(|(a, _, _)| a.is_err()) {
      return selectors
        .into_iter()
        .filter_map(|(applies, _, _)| applies.err())
        .flatten()
        .collect::<Vec<_>>()
        .pipe(|e| Err(helpers::ParseError::Selector(MultipleErrors::from(e))));
    }
    let selectors = selectors
      .into_iter()
      .filter_map(|(applies, selector, capabilities)| if applies.unwrap() { Some((selector, capabilities)) } else { None });
    let (globals, selectors): (Vec<_>, Vec<_>) = selectors.partition_map(|(selector, capability)| if selector.is_global() { Either::Left } else { Either::Right }(capability));
    let mut capabilities = None::<CapabilitiesCanonical>;

    for capability in globals {
      capabilities = capabilities.merge(capability.into());
    }

    for capability in selectors {
      capabilities = capabilities.merge(capability.into());
    }

    capabilities.unwrap_or_default().pipe(Ok)
  }
}

impl Merge<Option<CapabilitiesCanonical>> for Option<CapabilitiesCanonical> {
  #[cfg_attr(feature = "profiling", instrument)]
  fn merge(self, merge: Option<CapabilitiesCanonical>) -> Self {
    if let Some(s) = self {
      if let Some(merge) = merge { s.merge(merge) } else { s }.into()
    } else {
      merge
    }
  }
}

impl Merge<Self> for CapabilitiesCanonical {
  #[cfg_attr(feature = "profiling", instrument)]
  fn merge(mut self, Self { mut links, installs, depends }: Self) -> Self {
    if let Some(self_links) = &mut self.links {
      if let Some(merge_links) = &mut links {
        for (source_path, link_config) in &mut *merge_links {
          if let Some(existing_config) = self_links.get_mut(source_path) {
            // Merge the target sets
            existing_config.targets.extend(link_config.targets.clone());
            // For link type override, prefer the new one if it's specified
            if link_config.link_type.is_some() {
              existing_config.link_type = link_config.link_type.clone();
            }
          } else {
            self_links.insert(source_path.clone(), link_config.clone());
          }
        }
      }
    } else {
      self.links = links;
    }

    if let Some(i) = &mut self.installs {
      if let Some(installs) = installs {
        if installs.is_none() {
          self.installs = None;
        } else {
          let cmd_outer: String;
          let mut depends_outer;

          match installs {
            InstallsCanonical::Full { cmd, depends } => {
              cmd_outer = cmd;
              depends_outer = depends;
            }
            InstallsCanonical::None(_) => unreachable!(),
          }

          *i = match i {
            InstallsCanonical::None(_) => InstallsCanonical::Full {
              cmd: cmd_outer,
              depends: depends_outer,
            },
            InstallsCanonical::Full { depends, .. } => {
              depends_outer.extend(depends.clone());
              InstallsCanonical::Full {
                cmd: cmd_outer,
                depends: depends_outer,
              }
            }
          };
        }
      }
    } else {
      self.installs = installs;
    }

    if let Some(d) = &mut self.depends {
      if let Some(depends) = depends {
        d.extend(depends);
      }
    } else {
      self.depends = depends;
    }

    self
  }
}
