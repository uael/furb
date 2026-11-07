//! The name of a session, folded from the facts of its root.

use serde_json::{Value, json};

use super::{Named, said, word};

/// The name that facts fold, with the words of the rungs of the operator or not.
fn folded(facts: &[Value], words: bool) -> Named {
  let mut named = Named::default();
  for one in facts {
    named.hear(one, words);
  }
  named
}

#[test]
fn the_name_that_the_operator_said_wins_over_the_name_that_the_extension_gave() {
  let named = folded(
    &[
      json!(["rename_session", "rename_session1", "run1", "chain1"]),
      json!(["done", "rename_session1", "run1", "Mine"]),
      json!(["name_session", "name_session1", "run2", "chain1"]),
      json!(["done", "name_session1", "run2", "Given"]),
    ],
    false,
  );
  assert_eq!(
    (named.name(), named.said(), named.given()),
    (Some("Mine"), Some("Mine"), Some("Given"))
  );
}

#[test]
fn the_last_done_of_each_kind_names_the_session_and_an_empty_given_name_names_nothing() {
  let named = folded(
    &[
      json!(["name_session", "name_session1", "run1", "chain1"]),
      json!(["done", "name_session1", "run1", "First"]),
      json!(["name_session", "name_session2", "run2", "chain1"]),
      json!(["done", "name_session2", "run2", ""]),
      json!(["rename_session", "rename_session1", "run3", "chain1"]),
      json!(["rename_session", "rename_session2", "run4", "chain1"]),
      json!(["done", "rename_session1", "run3", "One"]),
      json!(["done", "rename_session2", "run4", "Two"]),
    ],
    false,
  );
  assert_eq!((named.said(), named.given()), (Some("Two"), Some("First")));
}

#[test]
fn an_empty_name_that_the_operator_said_takes_the_default_name_again() {
  let named = folded(
    &[
      json!(["name_session", "name_session1", "run1", "chain1"]),
      json!(["done", "name_session1", "run1", "Given"]),
      json!(["rename_session", "rename_session1", "run2", "chain1"]),
      json!(["done", "rename_session1", "run2", ""]),
    ],
    false,
  );
  assert_eq!(named.name(), Some(""));
}

#[test]
fn an_act_off_the_root_a_refused_act_and_an_act_not_done_name_nothing() {
  let named = folded(
    &[
      json!(["rename_session", "rename_session1", "run1", "chain2"]),
      json!(["done", "rename_session1", "run1", "Off"]),
      json!(["rename_session", "rename_session2", "run2", "chain1"]),
      json!(["done", "rename_session2", "run2", {"is": "Refused", "args": ["no"]}]),
      json!(["name_session", "name_session1", "run3", "chain1"]),
    ],
    false,
  );
  assert_eq!(named.name(), None);
}

#[test]
fn the_word_of_a_rung_of_the_operator_on_the_root_names_the_session_only_when_words_count() {
  let name = r#"say "it" \ twice"#;
  let facts = [
    json!(["rung", "rung1", "operator", "chain1", word("pf-1208"), "", ""]),
    json!(["rung", "rung2", "operator", "chain1", word(name), "", ""]),
    json!(["rung", "rung3", "rung2", "chain1", word("by a model"), "", ""]),
    json!(["rung", "rung4", "operator", "chain2", word("off the root"), "", ""]),
  ];
  assert_eq!(folded(&facts, true).said(), Some(name));
  assert_eq!(folded(&facts, false).said(), None);
}

#[test]
fn the_word_of_a_member_of_the_operator_on_the_root_names_the_session() {
  let facts = [
    json!(["rung", "rung1", "operator/uael", "chain1", word("by a member"), "", ""]),
    json!(["rung", "rung2", "operators", "chain1", word("by a stranger"), "", ""]),
  ];
  assert_eq!(folded(&facts, true).said(), Some("by a member"));
}

#[test]
fn the_word_that_names_a_session_says_its_name_and_no_other_word_does() {
  assert_eq!(said(&word("a \"b\" \u{e9}")).as_deref(), Some("a \"b\" \u{e9}"));
  assert_eq!(said(r#"act("rename_session", "", lambda id: (yield "done", id, "x")) + 1"#), None);
  assert_eq!(said("x = 1"), None);
}
