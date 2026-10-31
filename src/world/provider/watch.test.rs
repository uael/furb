//! The watch over the word of a reply.

use super::{Watch, broken, last, open, refused, settled};

/// A whole text less the marks that the watch drops, as [`super::bare`] gives it.
fn word(text: &str) -> Option<String> {
  super::bare(text).map(|(word, _)| word)
}

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
  watch.wrote("x = 1\ny = 2\nz");
  assert_eq!(watch.through(1), "x = 1");
  assert_eq!(watch.through(2), "x = 1\ny = 2");
}

#[test]
fn a_fence_around_a_word_goes_as_each_of_its_lines_ends() {
  let mut watch = Watch::default();
  assert_eq!(watch.wrote("```python\nx = 1\n"), None);
  assert!(watch.note().is_some());
  let end = watch.wrote("y = 2\n```").expect("the fence stands at the left edge");
  assert_eq!(watch.upto(end), "x = 1\ny = 2\n");
  assert_eq!(broken(watch.upto(end)), None);
}

#[test]
fn the_markup_of_a_tool_call_around_a_word_goes_as_each_of_its_lines_ends() {
  let mut watch = Watch::default();
  watch.wrote(
    "<invoke name=\"python\">\n<parameter name=\"code\">x = 1\ny = 2</parameter>\n</invoke>\nz",
  );
  assert!(watch.note().is_some());
  assert_eq!(watch.through(2), "x = 1\ny = 2");
  let mut plain = Watch::default();
  plain.wrote("x = 1\n");
  assert!(plain.note().is_none());
}

#[test]
fn a_text_loses_the_marks_of_a_fence_or_of_a_tool_call_around_it_and_keeps_all_else() {
  assert_eq!(word("```python\nclose(3)\n```").as_deref(), Some("close(3)"));
  assert_eq!(word("```\nclose(3)").as_deref(), Some("close(3)"), "a fence with no close goes");
  let call =
    "<invoke name=\"python\">\n<parameter name=\"code\">\nclose(3)\n</parameter>\n</invoke>";
  assert_eq!(word(call).as_deref(), Some("close(3)"));
  let inline = "<function_calls>\n<invoke name=\"python\">\n<parameter name=\"code\">x = 1\nclose(x)</parameter>\n</invoke>\n</function_calls>";
  assert_eq!(word(inline).as_deref(), Some("x = 1\nclose(x)"));
  assert_eq!(word("<invoke name=\"bash\">").as_deref(), Some(""), "a word of marks alone is empty");
  let quoted = "```python\n<s:doc>\n```sh\nls\n```\n</s:doc>\nclose(doc)\n```";
  assert_eq!(
    word(quoted).as_deref(),
    Some("<s:doc>\n```sh\nls\n```\n</s:doc>\nclose(doc)"),
    "the fences of a quote stay"
  );
  assert_eq!(word("close(3)\n```"), None, "a fence that no mark opened stays");
  assert_eq!(word("x = 1\n<invoke name=\"a\">"), None, "a tag after a line of the word stays");
  assert_eq!(
    word("<s:invoke>\nhi\n</s:invoke>\nclose(invoke)"),
    None,
    "the mark of a quote is no tag"
  );
  assert_eq!(word("```js and more\nclose(3)"), None, "a fence with words after it stays");
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

#[test]
fn a_quote_stands_open_as_the_table_of_the_quotes_says() {
  let rows: Vec<serde_json::Value> =
    serde_json::from_str(include_str!("../../../test/quotes.json")).unwrap();
  for row in rows {
    let word = row["word"].as_str().unwrap();
    assert_eq!(open(word), row["open"].as_bool().unwrap(), "{word:?}");
  }
}

#[test]
fn a_close_mark_inside_a_line_of_a_quote_ends_nothing() {
  assert!(open("<s:old>\nlet end = \"</s:old>\" in it\nmore\n"));
  assert!(open("<s:old>\nthe mark </s:old> and more\n"));
  assert!(!open("<s:old>\nthe mark </s:old> and more\n</s:old>\n"));
  assert!(!open("<s:old>text</s:old>"));
  assert!(open("x = 1\n<s:b>\nHello</s:a>\n"));
  assert!(!open("x = \"<s:b>\" + y\n"));
  assert!(!open("<s:old-text>\nHello\n"));
}

#[test]
fn a_statement_that_runs_at_once_is_settled_over_each_of_its_lines() {
  assert_eq!(settled("x = 1\nif x:\n  y = (\n    2)\n"), vec![(1, 1), (2, 4)]);
}

#[test]
fn a_statement_that_holds_a_part_that_runs_later_or_never_is_not_settled() {
  let word = "def f():\n  return g()\nclass C: ...\nh = lambda: g()\nn = [g() for _ in ()]\nm: G = 1\nz = 1\n";
  assert_eq!(settled(word), vec![(7, 7)]);
}

#[test]
fn a_word_that_is_not_python_has_no_settled_statement() {
  assert_eq!(settled("x = (1,\n"), vec![]);
}

#[test]
fn the_word_is_refused_after_the_first_settled_statement_that_holds_a_finding() {
  let settled = [(1, 1), (3, 5), (6, 6)];
  let found =
    ["line 2: in a def".to_owned(), "line 4: unknown name".to_owned(), "line 6: x".to_owned()];
  assert_eq!(refused(&settled, &found), Some(5));
  assert_eq!(refused(&settled, &found[..1]), None);
}

#[test]
fn a_part_is_the_word_that_is_new_since_the_parts_given_and_counts_the_characters_before_it() {
  let mut watch = Watch::default();
  assert_eq!(watch.part("x = 1\n"), Some(("x = 1\n", 0)));
  assert_eq!(watch.ran(), None, "no part was given yet");
  watch.wrote("é = 1\ny");
  watch.gave("é = 1\n", "é = 1\n".len());
  assert_eq!(watch.ran(), Some("é = 1\n"));
  assert_eq!(
    watch.part("é = 1\ny = 2\n"),
    Some(("y = 2\n", 6)),
    "python counts characters, not bytes"
  );
  assert_eq!(watch.part("é = 1\n\n"), None, "a part holds a statement");
  assert_eq!(watch.part("z = 3\n"), None, "a word that does not follow what was given has no part");
  watch.untaken();
  assert_eq!(watch.part("é = 1\ny = 2\n"), None, "a word that no Kernel takes is given whole");
}

#[test]
fn a_line_that_opens_a_clause_of_the_statement_before_it_holds_that_statement() {
  let mut watch = Watch::default();
  let end = watch.wrote("if x:\n  y = 1\nelse:\n").expect("a line at the left edge");
  assert!(watch.clause(end));
  assert_eq!(last("z = 0\nif x:\n  y = 1\n"), Some(2));
  assert_eq!(watch.start(3), "if x:\n  y = 1\n".len());
  assert_eq!(watch.start(1), 0);
  let mut other = Watch::default();
  let end = other.wrote("x = 1\nelsewhere = 2\n").expect("a line at the left edge");
  assert!(!other.clause(end), "a name that starts with a keyword opens no clause");
}

#[test]
fn a_first_line_that_starts_with_a_lone_equal_sign_binds_an_underscore() {
  let (named, note) =
    super::bare("= await bash(\"ls\")\nclose(1)").expect("the binding takes a name");
  assert_eq!(named, "_ = await bash(\"ls\")\nclose(1)");
  assert!(note.contains("a binding with no name"), "{note}");
  let (named, _) =
    super::bare("```python\n= read(\"a\")\n```").expect("a fence and a binding of no name");
  assert_eq!(named, "_ = read(\"a\")");
  assert_eq!(word("x == 1"), None, "a comparison is no binding");
  assert_eq!(word("== 1"), None, "a lone == is no binding of no name");
  assert_eq!(word("x = 1\n= 2"), None, "a later line keeps its fault for the gate");
  let mut watch = Watch::default();
  let end = watch.wrote("= await bash(\"ls\")\nclose(1)").expect("a line at the left edge");
  assert_eq!(watch.upto(end), "_ = await bash(\"ls\")\n");
  assert!(watch.note().is_some());
}

#[test]
fn a_break_of_html_alone_goes_before_the_word() {
  assert_eq!(word("<br>").as_deref(), Some(""), "a word of a break alone is empty");
  assert_eq!(word("<br/>\n```\nclose(1)").as_deref(), Some("close(1)"));
  assert_eq!(word("<BR />\nclose(1)").as_deref(), Some("close(1)"));
  assert_eq!(word("close(1)\n<br>"), None, "a break after a line of the word stays for the gate");
}

#[test]
fn the_close_of_a_tag_of_a_tool_call_alone_after_a_word_of_python_goes() {
  assert_eq!(word("x = 1\nclose(x)\n</parameter>").as_deref(), Some("x = 1\nclose(x)"));
  assert_eq!(word("x = 1\n</invoke>\n</function_calls>").as_deref(), Some("x = 1"));
  assert_eq!(
    word("<s:doc>\nhi\n</s:doc>\nclose(doc)\n</parameter>").as_deref(),
    Some("<s:doc>\nhi\n</s:doc>\nclose(doc)"),
    "a quote that ends is python"
  );
  assert_eq!(word("x = \"\"\"\n</parameter>\n\"\"\""), None, "the close in a string stays");
  assert_eq!(
    word("<s:doc>\n</parameter>\n</s:doc>\nclose(doc)"),
    None,
    "the close in a quote stays"
  );
  assert_eq!(word("x = (\n</parameter>"), None, "the close after a word that is not python stays");
}
