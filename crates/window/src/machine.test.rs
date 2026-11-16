//! The program that serves the daemon, which no door of the window reaches: a window with no furb
//! ends before it draws.

use std::{env, path::Path};

use super::furb;

#[test]
fn a_window_with_no_furb_names_where_it_looked_and_what_to_run() {
  let beside = env::temp_dir().join("furb-window-test-no-such-folder").join("furb");
  let no = furb(beside.clone(), Path::new("").as_os_str()).expect_err("no furb stands there");
  let said = format!(
    "no program furb stands at {} or on PATH, so nothing serves the daemon: build it in the tree of the window with `cargo build -p furb-cli`, or install furb",
    beside.display()
  );
  assert_eq!(no.to_string(), said);
}
