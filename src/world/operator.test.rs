//! The operator: a line it writes back, read as a value of the shape a prompt wants.

use super::{answered, answers};

/// What a line comes to, as python shows it, or why it is none.
fn read(shape: &str, line: &str) -> String {
  answered(shape, line).map_or_else(|no| no.to_string(), |value| value.py_repr())
}

#[test]
fn a_line_is_read_as_the_shape_a_prompt_wants() {
  assert_eq!(read("None", "anything"), "None");
  assert_eq!(read("str", " as typed "), "' as typed '");
  assert_eq!(read("int", " 12 "), "12");
  assert_eq!(read("float", "2"), "2.0");
  assert_eq!((read("bool", "YES"), read("bool", "n")), ("True".to_owned(), "False".to_owned()));
}

#[test]
fn a_line_that_is_no_value_of_the_shape_is_refused() {
  assert_eq!(read("int", "many"), "Refused: \"many\" is no int");
  assert_eq!(read("int", "0x10"), "Refused: \"0x10\" is no int");
  assert_eq!(read("bool", "maybe"), "Refused: \"maybe\" is neither yes nor no");
  assert_eq!(read("list", "{}"), "Refused: \"{}\" is no list");
  assert_eq!(read("set", "{}"), "Refused: the operator answers no set");
}

#[test]
fn a_list_and_a_dict_are_json_whose_every_number_is_exact() {
  assert_eq!(
    read("list", "[2.0, 3, 9223372036854775808]"),
    "[2.0, 3, {'is': 'int', 'args': ['9223372036854775808']}]"
  );
  let marked = read("dict", r#"{"a": {"is": "str"}}"#);
  assert_eq!(marked, "{'a': {'is': 'dict', 'args': [[('is', 'str')]]}}");
}

#[test]
fn a_literal_is_one_of_its_values_which_a_line_names_as_python_shows_it_or_as_its_text() {
  let shape = r#"Literal['a', "it's", 'b\n', 3, True, None]"#;
  assert!(answers(shape) && answers("int") && !answers("set") && !answers("Option"));
  assert_eq!(read(shape, " a "), "'a'");
  assert_eq!(read(shape, "'a'"), "'a'");
  assert_eq!(read(shape, "it's"), "\"it's\"");
  assert_eq!(read(shape, r"'b\n'"), "'b\\n'");
  assert_eq!(
    (read(shape, "3"), read(shape, "True"), read(shape, "None")),
    ("3".into(), "True".into(), "None".into())
  );
  assert_eq!(read(shape, "c"), format!("Refused: \"c\" is none of {shape}"));
  assert_eq!(read(shape, "1"), format!("Refused: \"1\" is none of {shape}"));
  for shape in ["Literal[]", "Literal[1.5]", "Literal[b'a']", "Literal['a'] | set", "Literal['a"] {
    assert!(!answers(shape), "{shape}");
    assert_eq!(read(shape, "a"), format!("Refused: the operator answers no {shape}"));
  }
}

#[test]
fn a_list_a_dict_and_a_tuple_hold_items_of_their_shapes() {
  assert_eq!(read("list[str]", r#"["a", "b"]"#), "['a', 'b']");
  assert_eq!(read("list[int]", "[1, 2, 3]"), "[1, 2, 3]");
  assert_eq!(read("list[float]", "[1, 2.5]"), "[1.0, 2.5]");
  assert_eq!(read("list[list[bool]]", "[[true], []]"), "[[True], []]");
  assert_eq!(read("list[str]", "[]"), "[]");
  assert_eq!(read("dict[str, int]", r#"{"a": 1}"#), "{'a': 1}");
  assert_eq!(read("tuple[int, str]", r#"[1, "a"]"#), "{'is': 'tuple', 'args': [[1, 'a']]}");
  assert_eq!(read("list[Literal['a', 'b']]", r#"["b", "a"]"#), "['b', 'a']");
  assert_eq!(read("list[int | None]", "[1, null]"), "[1, None]");
  assert_eq!(
    read("list[int]", "[9223372036854775808]"),
    "[{'is': 'int', 'args': ['9223372036854775808']}]"
  );
}

#[test]
fn an_item_that_has_not_the_shape_of_its_place_refuses_the_line() {
  assert_eq!(read("list[str]", "[1]"), "Refused: \"[1]\" is no list[str]");
  assert_eq!(read("list[int]", "[1.5]"), "Refused: \"[1.5]\" is no list[int]");
  assert_eq!(read("list[str]", "a, b"), "Refused: \"a, b\" is no list[str]");
  assert_eq!(
    read("dict[str, int]", r#"{"a": "b"}"#),
    "Refused: \"{\\\"a\\\": \\\"b\\\"}\" is no dict[str, int]"
  );
  assert_eq!(read("tuple[int, str]", "[1]"), "Refused: \"[1]\" is no tuple[int, str]");
  assert_eq!(
    read("tuple[int, str]", r#"[1, "a", 2]"#),
    "Refused: \"[1, \\\"a\\\", 2]\" is no tuple[int, str]"
  );
  assert_eq!(
    read("list[Literal['a']]", r#"["b"]"#),
    "Refused: \"[\\\"b\\\"]\" is no list[Literal['a']]"
  );
  for shape in [
    "list[Option]",
    "dict[int, str]",
    "tuple[]",
    "tuple[int, ...]",
    "list[str",
    "set[str]",
    "Option | None",
  ] {
    assert!(!answers(shape), "{shape}");
  }
}

#[test]
fn a_union_is_its_first_shape_that_reads_the_line_and_none_for_no_word() {
  assert_eq!(
    (read("int | None", " 4 "), read("int | None", " ")),
    ("4".to_owned(), "None".to_owned())
  );
  assert_eq!(read("str | None", "words"), "'words'");
  assert_eq!((read("int | str", "4"), read("int | str", "four")), ("4".into(), "'four'".into()));
  assert_eq!((read("str | int", "4"), read("bool | None", "no")), ("'4'".into(), "False".into()));
  assert_eq!(read("Literal['a'] | None", "a"), "'a'");
  assert_eq!(read("list[str] | None", r#"["a"]"#), "['a']");
  assert_eq!(read("int | None", "four"), "Refused: \"four\" is no int | None");
}
