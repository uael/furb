//! The paths and the texts of the files, proved where no life reaches the home of the user.

use std::{fs, path::PathBuf};

use super::{homed, read};

#[test]
fn a_path_that_is_a_tilde_or_starts_with_one_and_a_slash_resolves_against_the_home_of_the_user() {
  let home = PathBuf::from("/h");
  let at = |path: &str| homed(path, Some(home.clone())).map(Result::unwrap);
  assert_eq!(at("~"), Some(home.clone()));
  assert_eq!(at("~/a.txt"), Some(home.join("a.txt")));
  assert_eq!(at("~/x/../a.txt"), Some(home.join("a.txt")));
  assert_eq!(at("~a.txt"), None, "a name that starts with a tilde is a name");
  assert_eq!(at("a/~/b"), None);
  let none = homed("~/a.txt", None).expect("a path of the home").unwrap_err();
  assert_eq!(none.message(), "There is no home of the user for ~/a.txt.");
}

#[test]
fn a_read_gives_the_whole_text_of_a_file_of_any_size_and_refuses_what_is_no_file() {
  let at = std::env::temp_dir().join(format!("furb-files-large-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  let content = "x".repeat(1_000_000) + "\n";
  fs::write(at.join("large.txt"), &content).unwrap();
  assert_eq!(read(&at.join("large.txt")).unwrap().content, content);
  assert_eq!(read(&at).unwrap_err().message(), format!("Read needs a text file: {}", at.display()));
  fs::remove_dir_all(&at).unwrap();
}
