//! The command that runs the program of a kit.

use std::path::{Path, PathBuf};

use super::{UNCACHED, commanded};

#[test]
fn a_module_that_uv_runs_takes_the_cache_of_uv_whatever_a_project_above_it_says() {
  let program = Path::new("/kits/web/process.py");
  let script: &[&str] = &["run", "--quiet", "--script"];
  let command = commanded(Some((PathBuf::from("uv"), script)), program);
  let said =
    command.as_std().get_envs().find(|(name, _)| *name == UNCACHED).and_then(|(_, value)| value);
  assert_eq!(said.and_then(|one| one.to_str()), Some("false"));
  let plain = commanded(None, program);
  assert!(
    plain.as_std().get_envs().all(|(name, _)| name != UNCACHED),
    "a program on no runner takes no setting of uv"
  );
}
