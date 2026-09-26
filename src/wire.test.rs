//! The wire, proved where JavaScript cannot see it: a line of the record read with every number exact.

use serde_json::{Value, json, value::RawValue};

use super::{decoded, parsed};
use crate::value::{Fault, IS, field};

fn decode(line: &str) -> Result<Value, Fault> {
  decoded(serde_json::from_str::<&RawValue>(line).expect("a line of json"), 0)
}

#[test]
fn a_line_of_the_record_keeps_every_number_exact_or_refuses_it() {
  let key = "$serde_json::private::Number";
  assert_eq!(decode(&format!("{{{key:?}:\"10\"}}")).unwrap(), json!({ key: "10" }));
  for line in ["9007199254740993", "99999999999999999999999999999999"] {
    assert!(decode(line).unwrap_err().to_string().contains("safe integer"), "{line}");
  }
  let text = "\"99999999999999999999999999999999\"";
  assert_eq!(decode(text).unwrap(), json!("99999999999999999999999999999999"));
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
