//! The window of the checkout of the build, which furb looks for last: the checkout is fixed when
//! furb is built, so no process can name another one, and the suite proves the places before it.

use std::{
  ffi::OsString,
  fs,
  path::{Path, PathBuf},
  process,
};

#[path = "../../../tests/common/yard.rs"]
mod yard;

use super::found;
use crate::error::CliError;

/// A yard of the test: the folder of a furb, a directory for its PATH, and one for its checkout.
struct Yard {
  furb: PathBuf,
  bin: PathBuf,
  checkout: PathBuf,
  /// The folder of the yard, which goes with it.
  _at: yard::Yard,
}

impl Yard {
  fn new(name: &str) -> Self {
    let at = yard::Yard::new(format!("furb-window-{name}-{}", process::id()));
    let (furb, bin, checkout) = (at.join("furb"), at.join("bin"), at.join("checkout"));
    for folder in [&furb, &bin, &checkout] {
      fs::create_dir_all(folder).expect("a folder of the test");
    }
    Self { furb, bin, checkout, _at: at }
  }

  /// A program of the window in this folder.
  fn window(folder: &Path) -> PathBuf {
    fs::create_dir_all(folder).expect("the folder of the window");
    let at = folder.join(if cfg!(windows) { "furb-window.exe" } else { "furb-window" });
    fs::write(&at, "#!/bin/sh\n").expect("the program is written");
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      fs::set_permissions(&at, fs::Permissions::from_mode(0o755)).expect("the program runs");
    }
    at
  }

  /// The checkout of the window, with no build.
  fn checkout(&self) {
    let manifest = self.checkout.join("crates/window/Cargo.toml");
    fs::create_dir_all(manifest.parent().expect("a folder")).expect("the folder of the window");
    fs::write(&manifest, "").expect("the manifest of the window is written");
  }

  fn found(&self, named: Option<&str>) -> Result<PathBuf, CliError> {
    found(named.map(OsString::from), Some(&self.furb), self.bin.as_os_str(), &self.checkout)
  }
}

#[test]
fn the_window_that_the_checkout_built_comes_last() {
  let yard = Yard::new("checkout");
  yard.checkout();
  let built = Yard::window(&yard.checkout.join("target/debug"));
  assert_eq!(yard.found(None).unwrap(), built);
}

#[test]
fn a_checkout_says_how_to_build_its_window() {
  let yard = Yard::new("unbuilt");
  yard.checkout();
  let unbuilt = yard.found(None).unwrap_err().to_string();
  assert!(unbuilt.contains("run `cargo build -p furb-window` there"), "{unbuilt}");
}

#[test]
fn with_no_window_furb_says_how_to_get_one() {
  let yard = Yard::new("none");
  let none = yard.found(None).unwrap_err();
  assert!(matches!(none, CliError::NoWindow));
  assert!(none.to_string().starts_with("no window is found: set FURB_WINDOW"), "{none}");
}
