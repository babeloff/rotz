use std::path::PathBuf;

use speculoos::{assert_that, prelude::*};
use tap::Tap;
use velcro::hash_set;

use super::{get_handlebars, get_parameters};
use crate::encryption::LinkConfig;

#[test]
fn structure() {
  let dot = crate::parse!("yaml", &get_handlebars(), &get_parameters());

  let expected_config_01 = LinkConfig {
    targets: hash_set![PathBuf::from("v01")],
    link_type: None,
  };
  let expected_config_02 = LinkConfig {
    targets: hash_set![PathBuf::from("v02a"), PathBuf::from("v02b")],
    link_type: None,
  };

  assert_that!(dot.links)
    .is_some()
    .tap_mut(|l| l.contains_entry(PathBuf::from("k01"), &expected_config_01))
    .tap_mut(|l| l.contains_entry(PathBuf::from("k02"), &expected_config_02));
}
