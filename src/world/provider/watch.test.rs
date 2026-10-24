//! The watch over the word of a reply.

use super::{Watch, broken, open};

#[test]
fn a_fence_that_opens_a_word_is_a_fault_at_its_line() {
  assert_eq!(broken("```python\n"), Some(1));
  assert_eq!(broken("x = 1\n```\n"), Some(2));
}

#[test]
fn prose_before_the_code_is_a_fault_at_its_line() {
  assert_eq!(broken("Here is the code:\n"), Some(1));
  assert_eq!(broken("x = 1\nI will read the file now\n"), Some(2));
}

#[test]
fn a_statement_that_stands_whole_is_no_fault() {
  assert_eq!(broken("x = 1\nif x:\n  y = 2\n"), None);
  assert_eq!(broken("a = await bash(\"ls\")\n"), None);
}

#[test]
fn a_statement_that_the_next_text_may_finish_is_no_fault() {
  for word in
    ["if x:\n", "x = (1,\n", "x = \"\"\"\ntext\n", "def f(a,\n", "x = 1 + \\\n", "try:\n  a\n"]
  {
    assert_eq!(broken(word), None, "{word:?}");
  }
}

#[test]
fn a_fault_before_the_end_stays_whatever_follows() {
  assert_eq!(broken("x = (1,\n y = 2\n"), Some(2));
  assert_eq!(broken("x = 1\nif x:\npass\n"), Some(3));
}

#[test]
fn the_statements_to_read_end_where_the_last_line_at_the_left_edge_starts() {
  let mut watch = Watch::default();
  assert_eq!(watch.wrote("x = 1\n"), None);
  let end = watch.wrote("if x:\n  y").expect("a line stands at the left edge");
  assert_eq!(watch.upto(end), "x = 1\n");
  let end = watch.wrote(" = 2\nz").expect("a line stands at the left edge");
  assert_eq!(watch.upto(end), "x = 1\nif x:\n  y = 2\n");
  assert_eq!(watch.wrote(" = 3\n"), None);
  let end = watch.wrote("w = 4\n").expect("a line stands at the left edge");
  assert_eq!(watch.upto(end), "x = 1\nif x:\n  y = 2\nz = 3\n");
}

#[test]
fn a_word_is_cut_after_the_end_of_a_line_of_it() {
  let mut watch = Watch::default();
  watch.wrote("```python\nx = 1\ny");
  assert_eq!(watch.through(1), "```python");
  assert_eq!(watch.through(2), "```python\nx = 1");
  assert_eq!(watch.through(3), "```python\nx = 1\ny");
}

#[test]
fn a_word_forgotten_starts_again() {
  let mut watch = Watch::default();
  watch.wrote("x = 1\ny = 2\n");
  watch.forget();
  assert_eq!(watch.wrote("z = 3\n"), None);
  assert_eq!(watch.through(1), "z = 3");
}

#[test]
fn a_quote_stands_open_from_its_open_mark_to_its_close_mark() {
  assert!(!open("x = 1\n"));
  assert!(open("<s:text>\nHello\n"));
  assert!(!open("<s:text>\nHello\n</s:text>\n"));
  assert!(open("<s:a>\n</s:a>\n<s:b>\nHello\n"));
}
