//! The wire of a host, through `furb::wire`: a line of the record read with every number exact.

use serde_json::Value;

use furb::wire::{outward, parsed};

/// What a line of JSON comes in as, as python writes it.
fn read(line: &str) -> String {
  parsed(line).expect("a line of JSON").as_ref().py_repr()
}

#[test]
fn a_line_of_the_record_keeps_every_number_exact_or_refuses_it() {
  let key = "$serde_json::private::Number";
  assert_eq!(read(&format!("{{{key:?}:\"10\"}}")), format!("{{'{key}': '10'}}"));
  for line in ["9007199254740993", "99999999999999999999999999999999"] {
    let refused = parsed(line).expect_err("an integer past the safe ones").to_string();
    assert!(refused.contains("safe integer"), "{line}: {refused}");
  }
  let text = "\"99999999999999999999999999999999\"";
  assert_eq!(read(text), "'99999999999999999999999999999999'");
}

#[test]
fn a_value_of_json_comes_in_with_every_number_exact() {
  assert_eq!(read("[1, \"one\", null]"), "[1, 'one', None]");
  // A float and a fault come in as the marks that the wire gives them, which a host reads.
  assert_eq!(read("2.0"), "{'is': 'float', 'args': ['2']}");
  assert_eq!(
    read("{\"is\": \"Refused\", \"args\": [\"no\"]}"),
    "{'is': 'Refused', 'args': ['no']}"
  );
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
    let serde: Value = serde_json::from_str(line).expect("a line of JSON");
    assert_eq!(outward(parsed(line).expect("a line of JSON").as_ref()), serde, "{line}");
  }
}
