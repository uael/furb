//! The one path of the files that no door reaches: a user with no home, which no machine of a test
//! is. The suite of the world proves the rest through a life.

use super::homed;

#[test]
fn a_path_of_the_home_of_a_user_with_no_home_is_refused() {
  let none = homed("~/a.txt", None).expect("a path of the home").unwrap_err();
  assert_eq!(none.message(), "There is no home of the user for ~/a.txt.");
}
