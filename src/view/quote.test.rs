//! The quotes of a word, as each window reads them.

use super::{python_string, quotes, writing};

#[test]
fn a_text_gives_each_quote_that_closes_with_the_binding_that_it_makes() {
  let found =
    quotes("#read1\n<s:read1_text>\nit's\n</s:read1_text>\nx = 1\n<s:open>\nno close", false);
  assert_eq!(found.len(), 1, "{found:?}");
  let one = &found[0];
  assert_eq!(
    (one.start, one.name.as_str(), one.body.as_str(), one.end, one.closed),
    (1, "read1_text", "it's\n", 3, true)
  );
  assert_eq!(one.binding, "read1_text = \"it's\\n\"");
  let inline = quotes("<s:a>one</s:a>", false);
  assert_eq!((inline[0].body.as_str(), inline[0].end), ("one", 0));
}

#[test]
fn a_quote_that_a_model_still_writes_stands_as_its_binding_and_drops_the_part_of_its_close_mark_that_came()
 {
  let shown = writing("x = 1\n<s:note>\nhello\nwor</s:no", "");
  assert_eq!(shown.lines, "x = 1\nnote = 'hello\\nwor'");
  assert_eq!((shown.tail.as_str(), shown.numbers.clone(), shown.count), ("", vec![1, 2], 4));
  let thinks = writing("", "I look\nat the tests\n\n");
  assert_eq!((thinks.lines.as_str(), thinks.tail.as_str(), thinks.count), ("", "at the tests", 0));
  let begun = writing("a = 1\nfor x in a:\n  b = await bash(", "");
  assert_eq!(
    (begun.lines.as_str(), begun.tail.as_str()),
    ("a = 1\nfor x in a:", "b = await bash(")
  );
}

#[test]
fn a_string_of_python_escapes_as_repr_does() {
  assert_eq!(python_string("it's"), "\"it's\"");
  assert_eq!(python_string("a\tb\\\x01'\""), "'a\\tb\\\\\\x01\\'\"'");
}
