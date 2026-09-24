use super::*;

fn worded(source: &str) -> String {
  word(source).unwrap()
}

#[test]
fn a_word_without_an_import_of_furb_is_itself() {
  let plain = "import re\nfrom dataclasses import dataclass\n\nx = 1\n";
  assert_eq!(worded(plain), plain);
  assert_eq!(worded("x = 1"), "x = 1");
}

#[test]
fn a_top_level_import_from_furb_becomes_an_empty_line_so_every_line_keeps_its_number() {
  let module = "import re\nfrom furb.engine import ask\nfrom furb.builtin.files import Text\n\
    from furb.extensions.x import y\nfrom furb import engine\nx = ask\n";
  assert_eq!(worded(module), "import re\n\n\n\n\nx = ask\n");
  assert_eq!(worded(module).lines().count(), module.lines().count());
}

#[test]
fn a_parenthesized_import_over_several_lines_becomes_as_many_empty_lines() {
  assert_eq!(worded("from furb.engine import (\n  ask,\n  tell,\n)\nx = ask\n"), "\n\n\n\nx = ask\n");
}

#[test]
fn a_backslash_continued_import_becomes_as_many_empty_lines() {
  assert_eq!(worded("from furb.engine import ask, \\\n  tell\nx = ask\n"), "\n\nx = ask\n");
}

#[test]
fn an_import_of_another_package_a_relative_import_and_import_furb_stay() {
  let module = "from furbish import x\nfrom .engine import y\nimport furb\nimport furb.engine\n";
  assert_eq!(worded(module), module);
}

#[test]
fn an_import_of_furb_inside_a_function_or_a_block_stays() {
  let module = "def f():\n  from furb.engine import ask\n  return ask\nif True:\n  from furb import engine\n";
  assert_eq!(worded(module), module);
}

#[test]
fn an_import_that_shares_its_line_takes_its_semicolon_with_it() {
  assert_eq!(worded("from furb.engine import ask; x = 1\n"), "x = 1\n");
  assert_eq!(worded("x = 1; from furb.engine import ask\n"), "x = 1\n");
}

#[test]
fn a_line_end_is_lf_on_every_machine() {
  assert_eq!(worded("x = 1\r\ny = 2\r\n"), "x = 1\ny = 2\n");
  assert_eq!(worded("from furb.engine import ask\r\nx = ask\r\n"), "\nx = ask\n");
}

#[test]
fn every_other_byte_stays_as_it_is() {
  let module = "x = '''a  \nb\t'''   \nfrom furb.engine import ask\ny = 2  \n";
  assert_eq!(worded(module), "x = '''a  \nb\t'''   \n\ny = 2  \n");
  let plain = "x = 1   \n\n\ny = 2";
  assert_eq!(worded(plain), plain);
}

#[test]
fn a_module_that_does_not_parse_is_refused_with_its_line() {
  let refused = word("from furb.engine import ask\nx = 1\ny = (\n").unwrap_err();
  assert!(matches!(&refused, Error::Word { name, line: 3 | 4, .. } if name.is_empty()), "{refused:?}");
  assert!(Fault::from(refused).name == "Refused");
}

#[test]
fn the_builtins_are_files_bash_and_grant_and_bash_requires_files() {
  let held = builtins();
  assert_eq!(held.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["files", "bash", "grant"]);
  assert_eq!(held.iter().map(|one| one.requires.clone()).collect::<Vec<_>>(), [vec![], vec!["files".to_owned()], vec![]]);
  assert!(held.iter().all(|one| one.root.is_none() && one.world == Worlds::default() && one.tui.is_none()));
}

#[test]
fn the_word_of_each_builtin_is_the_word_of_the_file_the_package_ships() {
  for one in builtins() {
    let path = format!("{}/src/furb/builtin/{}.py", env!("CARGO_MANIFEST_DIR"), one.name);
    let shipped = std::fs::read_to_string(path).unwrap();
    assert_eq!(one.word, worded(&shipped));
    assert!(!one.word.contains("from furb"));
  }
  assert_eq!(words(&builtins()), builtins().into_iter().map(|one| one.word).collect::<Vec<_>>());
}

#[test]
fn the_missing_words_are_the_words_the_program_lacks_in_their_order() {
  let words = vec!["a = 1".to_owned(), "b = 2".to_owned(), "c = 3".to_owned()];
  assert_eq!(missing(&[], &words), ["a = 1", "b = 2", "c = 3"]);
  assert_eq!(missing(&["x = 0", "b = 2"], &words), ["a = 1", "c = 3"]);
  assert_eq!(missing(&["c = 3", "a = 1", "b = 2"], &words), Vec::<&str>::new());
}
