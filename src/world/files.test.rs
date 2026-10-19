//! The paths of the files, proved where no life reaches the home of the user.

use std::path::PathBuf;

use super::homed;

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
