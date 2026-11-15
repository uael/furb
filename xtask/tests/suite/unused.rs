//! The task `unused`, on a repository in a yard.

use std::process;

use crate::{ended, held, xtask, yard::Yard};

#[test]
fn a_public_item_that_only_a_test_names_is_listed_and_one_that_a_source_names_is_not() {
  let yard = Yard::new(format!("xtask-unused-{}", process::id()));
  let (rs, ts) = ("src/", "bind/typescript/src/");
  let files = [
    (
      format!("{rs}a.rs"),
      "pub fn used() {}\npub fn lonely() {}\npub struct Held(Inner);\npub struct Inner;\n",
    ),
    (format!("{rs}b.rs"), "fn main() { used(); let _ = Held; }\n"),
    (format!("{rs}a.test.rs"), "fn test() { lonely(); }\n"),
    (format!("{ts}one.ts"), "export const shown = 1;\nexport function hidden() {}\n"),
    (format!("{ts}two.ts"), "import { shown } from \"./one\";\n"),
  ];
  let files: Vec<(&str, &str)> = files.iter().map(|(file, text)| (file.as_str(), *text)).collect();
  held(&yard, &files);
  let (code, out, err) = ended(&mut xtask(&yard, ["unused"]));
  assert_eq!(
    (code, out),
    (Some(1), format!("{ts}one.ts:2: hidden\n{rs}a.rs:2: lonely\n")),
    "{err}"
  );
}
