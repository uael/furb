//! Where furb finds the TUI, in the order it looks, on a PATH and a checkout that a test makes.

use std::{ffi::OsString, fs, path::PathBuf};

use super::{NONE, found};

/// A yard of the test: a directory for its PATH and one for its checkout.
struct Yard {
  bin: PathBuf,
  checkout: PathBuf,
}

impl Yard {
  fn new(name: &str) -> Yard {
    let at = std::env::temp_dir().join(format!("furb-tui-{name}"));
    let _ = fs::remove_dir_all(&at);
    let (bin, checkout) = (at.join("bin"), at.join("checkout"));
    fs::create_dir_all(&bin).expect("a PATH of the test");
    fs::create_dir_all(&checkout).expect("a checkout of the test");
    Yard { bin, checkout }
  }

  /// A program of this name on the PATH of the yard.
  fn program(&self, name: &str) -> PathBuf {
    let at = self.bin.join(if cfg!(windows) { format!("{name}.cmd") } else { name.to_owned() });
    fs::write(&at, "#!/bin/sh\n").expect("the program is written");
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      fs::set_permissions(&at, fs::Permissions::from_mode(0o755)).expect("the program runs");
    }
    at
  }

  /// The TUI of the checkout, and its build when it is built.
  fn tui(&self, built: bool) -> PathBuf {
    let cli = self.checkout.join("tui/src/cli.ts");
    fs::create_dir_all(cli.parent().expect("a folder")).expect("the folder of the TUI");
    fs::write(&cli, "").expect("the TUI is written");
    if built {
      fs::create_dir_all(self.checkout.join("bind/typescript/dist")).expect("the build of the TUI");
    }
    cli
  }

  fn found(&self, named: Option<&str>) -> Result<Vec<OsString>, String> {
    found(named.map(OsString::from), self.bin.as_os_str(), &self.checkout)
  }
}

#[test]
fn the_program_that_furb_tui_names_comes_first() {
  let yard = Yard::new("named");
  yard.program("furb-tui");
  assert_eq!(yard.found(Some("/opt/tui")).unwrap(), [OsString::from("/opt/tui")]);
}

#[test]
fn the_furb_tui_on_path_comes_next() {
  let yard = Yard::new("path");
  let tui = yard.program("furb-tui");
  yard.tui(true);
  assert_eq!(yard.found(Some("")).unwrap(), [tui.into_os_string()]);
}

#[test]
fn the_tui_of_a_built_checkout_runs_on_the_bun_on_path() {
  let yard = Yard::new("checkout");
  let cli = yard.tui(true);
  let bun = yard.program("bun");
  assert_eq!(yard.found(None).unwrap(), [bun.into_os_string(), "run".into(), cli.into_os_string()]);
}

#[test]
fn a_checkout_says_what_its_tui_lacks() {
  let yard = Yard::new("lacks");
  yard.tui(false);
  assert!(yard.found(None).unwrap_err().contains("no bun is on PATH"));
  yard.program("bun");
  assert!(yard.found(None).unwrap_err().contains("run `bun install && bun run build` there"));
}

#[test]
fn with_no_tui_furb_says_how_to_get_one() {
  let yard = Yard::new("none");
  yard.program("bun");
  assert_eq!(yard.found(None).unwrap_err(), NONE);
}
