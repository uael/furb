//! The operator: a line it writes back, read as a value of the shape a thread wants.

use super::answered;

/// What a line comes to, as python shows it, or why it is none.
fn read(shape: &str, line: &str) -> String {
  answered(shape, line).map_or_else(|no| no.to_string(), |value| value.py_repr())
}

#[test]
fn a_line_is_read_as_the_shape_a_thread_wants() {
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
