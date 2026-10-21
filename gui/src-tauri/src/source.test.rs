use super::{clip, placed};

#[test]
fn each_act_stands_on_the_first_line_from_the_last_that_calls_its_verb() {
  let word = "a = read('x')\nb = bash('ls')\nc = read('y')";
  assert_eq!(placed(word, &["read", "bash", "read"]), vec![0, 1, 2]);
}

#[test]
fn a_verb_inside_a_longer_name_is_no_call_of_it() {
  assert_eq!(placed("reread('x')\nread('y')", &["read"]), vec![1]);
}

#[test]
fn a_long_line_is_cut_with_an_ellipsis() {
  assert_eq!(clip("abcdef", 4), "abc…");
  assert_eq!(clip("abc", 4), "abc");
}
