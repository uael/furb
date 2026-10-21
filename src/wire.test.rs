//! The wire, proved where JavaScript cannot see it: a line of the record read with every number exact.

use serde_json::Value;

use super::{inward, parsed};
use crate::value::{IS, field};

#[test]
fn a_line_of_the_record_keeps_every_number_exact_or_refuses_it() {
  let key = "$serde_json::private::Number";
  assert_eq!(
    parsed(&format!("{{{key:?}:\"10\"}}")).unwrap().py_repr(),
    format!("{{'{key}': '10'}}")
  );
  for line in ["9007199254740993", "99999999999999999999999999999999"] {
    assert!(parsed(line).unwrap_err().to_string().contains("safe integer"), "{line}");
  }
  let text = "\"99999999999999999999999999999999\"";
  assert_eq!(parsed(text).unwrap().py_repr(), "'99999999999999999999999999999999'");
}

#[test]
fn a_value_of_json_comes_in_with_every_number_exact() {
  assert_eq!(parsed("[1, \"one\", null]").unwrap().py_repr(), "[1, 'one', None]");
  let float = parsed("2.0").unwrap();
  assert_eq!(field(&float.as_ref(), IS).and_then(|one| one.as_str()), Some("float"));
  let fault = parsed("{\"is\": \"Refused\", \"args\": [\"no\"]}").unwrap();
  assert_eq!(field(&fault.as_ref(), IS).and_then(|one| one.as_str()), Some("Refused"));
  assert!(parsed("9007199254740993").unwrap_err().to_string().contains("safe integer"));
  assert!(parsed("{").is_err());
}

#[test]
fn a_value_of_json_comes_in_as_serde_reads_it() {
  let lines = [
    r#"[["done", "read1", "world", {"is": "Text", "path": "a\"b\\c\/d", "content": "\u00e9\n\t\ud83d\ude00 \u0000"}]]"#,
    " { \"a\" : [ 1 , -2 , 0.5 , -2.25e-3 , true , false , null , [ ] , { } ] ,\n\t\"b\" : \"\" , \"a\" : 3 } ",
    "[[[[\"deep\"]]], {\"é\": {\"\\u00e9\": \"a key with an escape\"}}, \"no escape é 😀\"]",
  ];
  for line in lines {
    let read = inward(&serde_json::from_str::<Value>(line).unwrap()).unwrap();
    assert_eq!(parsed(line).unwrap(), read, "{line}");
  }
}
