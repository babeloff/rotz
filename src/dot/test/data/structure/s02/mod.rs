use std::path::PathBuf;

use speculoos::{assert_that, prelude::*};
use tap::Tap;
use velcro::hash_set;

use crate::{encryption::LinkConfig, helpers::Select};

use super::{get_handlebars, get_parameters};

#[test]
fn structure() {
  let dot = crate::parse!("yaml", &get_handlebars(), &get_parameters());

  let expected_config_01 = LinkConfig {
    targets: hash_set![PathBuf::from("v01a"), PathBuf::from("v01b")],
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

  assert_that!(dot.installs)
    .is_some()
    .select_and(|i| &i.cmd, |mut c| c.is_equal_to("i01".to_owned()))
    .select_and(|i| &i.depends, |mut d| d.contains("d01".to_owned()));

  assert_that!(dot.depends).is_some().contains("d02".to_owned());
}
