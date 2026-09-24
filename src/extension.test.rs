use super::*;

#[test]
fn a_word_with_no_import_of_furb_is_its_own_word() {
  let plain = "import re\nfrom dataclasses import dataclass\n\nx = 1\n";
  assert_eq!(word(plain), plain);
  assert_eq!(word("x = 1"), "x = 1");
}

#[test]
fn every_top_level_import_from_furb_becomes_empty_lines() {
  let module = "import re\nfrom furb.engine import (\n  ask,\n  tell,\n)\nfrom furb.builtin.files import Text\nfrom furb import engine\n\nx = ask\n";
  assert_eq!(word(module), "import re\n\n\n\n\n\n\n\nx = ask\n");
  assert_eq!(word(module).lines().count(), module.lines().count());
}

#[test]
fn an_import_of_another_package_or_beneath_a_scope_stays() {
  let module = "from furbish import x\nfrom .engine import y\ndef f():\n  from furb.engine import ask\n  return ask\n";
  assert_eq!(word(module), module);
}

#[test]
fn an_import_that_shares_its_line_takes_its_semicolon_with_it() {
  assert_eq!(word("from furb.engine import ask; x = 1\n"), "x = 1\n");
  assert_eq!(word("x = 1; from furb.engine import ask\n"), "x = 1\n");
}

#[test]
fn a_file_python_cannot_parse_is_its_own_word() {
  let broken = "from furb.engine import ask\nx = (\n";
  assert_eq!(word(broken), broken);
}
