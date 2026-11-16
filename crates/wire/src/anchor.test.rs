use super::{Anchor, Where};

/// A message about lines reads back as the anchor that wrote it, one line or several.
#[test]
fn a_message_about_lines_reads_back_as_its_anchor() {
  for quote in ["one = 1", "one = 1\ntwo = 2\nthree = 3"] {
    let at = Where::Line { line: 12, before: false, quote: quote.to_owned() };
    let anchor = Anchor { path: "src/a.py".to_owned(), at };
    let said = anchor.said("why three?\n\nand not four");
    assert_eq!(Anchor::of(&said), Some((anchor.clone(), "why three?\n\nand not four")));
  }
  let at = Where::Line { line: 12, before: true, quote: "a\nb".to_owned() };
  let said = Anchor { path: "a.py".to_owned(), at }.said("x");
  assert_eq!(said, "On `a.py`, lines 12 to 13 before the change:\n> a\n> b\n\nx");
}
