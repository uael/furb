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
  assert!(answers(shape) && answers("int") && !answers("set"));
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
  for shape in ["Literal[]", "Literal[1.5]", "Literal[b'a']", "Literal['a'] | None", "Literal['a"] {
    assert!(!answers(shape), "{shape}");
    assert_eq!(read(shape, "a"), format!("Refused: the operator answers no {shape}"));
  }
}
