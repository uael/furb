//! The tests of the names that nothing but a test uses.

use std::collections::HashMap;

use super::unused;

#[test]
fn a_public_item_that_only_a_test_names_is_listed_and_one_that_a_source_names_is_not() {
  // The files stand in a map, under the folders that the check reads, and in no tree.
  let (rs, ts) = ("src/", "bind/typescript/src/");
  let text: HashMap<String, String> = [
    (
      format!("{rs}a.rs"),
      "pub fn used() {}\npub fn lonely() {}\npub struct Held(Inner);\npub struct Inner;\n",
    ),
    (format!("{rs}b.rs"), "fn main() { used(); let _ = Held; }\n"),
    (format!("{rs}a.test.rs"), "fn test() { lonely(); }\n"),
    (format!("{ts}one.ts"), "export const shown = 1;\nexport function hidden() {}\n"),
    (format!("{ts}two.ts"), "import { shown } from \"./one\";\n"),
  ]
  .into_iter()
  .map(|(path, body)| (path, body.to_owned()))
  .collect();
  let (found, _) = unused(&text, "").unwrap();
  assert_eq!(found, [format!("{ts}one.ts:2: hidden"), format!("{rs}a.rs:2: lonely")]);
}
