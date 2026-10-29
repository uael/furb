use std::{collections::BTreeMap, fs, path::PathBuf};

use monty_types::NamedValues;
use serde_json::{Map, Value, json};

use super::{Function, Shape, binding, quoted, safe, union};
use crate::{
  ENGINE, SHEET,
  gate::checked,
  sand::{Nobody, Sand},
  value::{Object, entry},
};

/// The word of the extension as far as a binding reads it, which the gate reads before a binding.
const WORD: &str = "McpResult = dict[str, Any]\n\n\ndef mcp_call(server: str, tool: str, arguments: dict[str, Any] | None = None) -> Act[McpResult]:\n  return act(\"mcp_call\", \"\", None, server, tool, arguments)\n";

/// What runs before a binding in the sandbox: a `mcp_call` that gives what a call sends.
const SENDS: &str = "from typing import Any, Literal\nimport json\nMcpResult = dict[str, Any]\n\n\nasync def mcp_call(server, tool, arguments):\n  return {'server': server, 'tool': tool, 'arguments': arguments}\n";

/// The keywords of JSON Schema that say the shape of a value, which a function that maps its tool exactly never sends
/// against: the others say a constraint on a value of the right shape, as a pattern or a minimum.
const SHAPED: [&str; 7] =
  ["type", "enum", "const", "required", "additionalProperties", "items", "prefixItems"];

/// The most calls of one function that a measure makes.
const CALLS: usize = 12;

/// The sheet of one word after a program, and how many lines stand above the word, as `furb.sheet` writes it.
fn sheet(program: &[&str], word: &str) -> (String, usize) {
  let code = "__engine = {}\nexec(__source, __engine)\n__sheet = {}\nexec(__sheet_source, __sheet)\n__program = {f'rung{n}': one for n, one in enumerate(__words, 1)}\n__sheet['sheet'](__engine, __program, __word, [])";
  let mut named = NamedValues::new();
  named.push("__source", Object::string(ENGINE));
  named.push("__sheet_source", Object::string(SHEET));
  named.push("__words", Object::list(program.iter().map(|one| Object::string(*one))));
  named.push("__word", Object::string(word));
  let got =
    Sand::new().run(code, named, &mut Nobody("the sheet")).expect("furb.sheet writes the sheet");
  let got = got.as_ref();
  let text = entry(&got, 0).and_then(|one| one.as_str()).expect("a sheet is a text").to_owned();
  let above =
    entry(&got, 1).and_then(|one| one.as_int()).expect("a sheet counts the lines above the word");
  (text, usize::try_from(above).expect("a count of lines is no negative number"))
}

/// What the gate finds on a word after a program, each by its line in the word.
fn found(program: &[&str], word: &str) -> Vec<String> {
  let (text, above) = sheet(program, word);
  checked(&text)
    .expect("the gate reads the sheet")
    .into_iter()
    .filter(|(line, _)| *line > above)
    .map(|(line, why)| format!("line {}: {why}", line - above))
    .collect()
}

/// What each call sends, in order, when the word of a binding runs in the sandbox and the calls run after it.
fn sent(word: &str, calls: &[String]) -> Vec<Value> {
  let made: String = calls.iter().map(|one| format!("__got.append({one})\n")).collect();
  let code = format!("{SENDS}{word}__got = []\n{made}json.dumps(__got)");
  let got = Sand::new()
    .run(&code, NamedValues::new(), &mut Nobody("a binding"))
    .unwrap_or_else(|fault| panic!("the binding runs in monty: {fault}"));
  let text = got.as_ref().as_str().expect("json.dumps gives a text").to_owned();
  serde_json::from_str(&text).expect("the calls give JSON")
}

/// A value of JSON as Python writes it.
fn python(value: &Value) -> String {
  match value {
    Value::Null => "None".to_owned(),
    Value::Bool(true) => "True".to_owned(),
    Value::Bool(false) => "False".to_owned(),
    Value::Number(number) => number.to_string(),
    Value::String(text) => quoted(text),
    Value::Array(items) => format!("[{}]", items.iter().map(python).collect::<Vec<_>>().join(", ")),
    Value::Object(fields) => format!(
      "{{{}}}",
      fields
        .iter()
        .map(|(key, one)| format!("{}: {}", quoted(key), python(one)))
        .collect::<Vec<_>>()
        .join(", ")
    ),
  }
}

/// The values that a measure gives a parameter of a shape: one of each member and of each value of a Literal.
fn samples(shape: &Shape) -> Vec<Value> {
  match shape {
    Shape::Any | Shape::None => vec![Value::Null],
    Shape::Str => vec![json!("a")],
    Shape::Int => vec![json!(1)],
    Shape::Float => vec![json!(1.5)],
    Shape::Bool => vec![json!(true)],
    Shape::Literal(values) => values.clone(),
    Shape::List(item) => samples(item).into_iter().map(|one| json!([one])).collect(),
    Shape::Dict(value) => match value.as_ref() {
      Shape::Any => vec![json!({})],
      other => samples(other).into_iter().map(|one| json!({ "a": one })).collect(),
    },
    Shape::Union(members) => members.iter().flat_map(samples).collect(),
  }
}

/// The calls of a function that a measure makes from its signature, each with the arguments that it must send: one
/// with its required parameters alone, by place, then one for each value that a parameter takes, with every
/// parameter, by place and by name in turn.
fn calls(server: &str, class: &str, one: &Function) -> Vec<(String, Value)> {
  let pools: Vec<Vec<Value>> = one.params.iter().map(|param| samples(&param.shape)).collect();
  let many = pools.iter().map(Vec::len).max().unwrap_or(0).clamp(1, CALLS);
  (0..=many)
    .map(|at| {
      let mut given = Vec::new();
      let mut arguments = Map::new();
      for (param, pool) in one.params.iter().zip(&pools) {
        if !param.required && at == 0 {
          continue;
        }
        let value = pool[at.saturating_sub(1) % pool.len()].clone();
        let written = python(&value);
        given.push(if param.required && at % 2 == 0 {
          written
        } else {
          format!("{}={written}", param.name)
        });
        if param.required || !value.is_null() {
          arguments.insert(param.key.clone(), value);
        }
      }
      let call = format!("await {class}.{}({})", one.name, given.join(", "));
      (call, json!({ "server": server, "tool": one.tool, "arguments": arguments }))
    })
    .collect()
}

/// The servers of the corpus, each with its name and its file.
fn corpus() -> Vec<(String, Value)> {
  let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/mcp-corpus");
  let mut files: Vec<PathBuf> = fs::read_dir(&folder)
    .expect("the corpus stands")
    .map(|one| one.expect("the corpus lists its files").path())
    .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
    .collect();
  files.sort();
  files
    .into_iter()
    .map(|path| {
      let text = fs::read_to_string(&path).expect("a file of the corpus reads");
      let file: Value = serde_json::from_str(&text).expect("a file of the corpus is JSON");
      (file["server"].as_str().expect("a file of the corpus names its server").to_owned(), file)
    })
    .collect()
}

/// The tools of a file of the corpus.
fn tools(file: &Value) -> &[Value] {
  file["tools"].as_array().map_or(&[], Vec::as_slice)
}

/// The shape and the reasons of looseness of one required argument `x` of the schema, with `$defs` beside it.
fn mapped(schema: Value, defs: Value) -> (String, Vec<String>) {
  let input =
    json!({ "type": "object", "properties": { "x": schema }, "required": ["x"], "$defs": defs });
  let made = binding("s", &[json!({ "name": "t", "inputSchema": input })]);
  let one = &made.functions[0];
  (one.params[0].shape.python(), one.loose.clone())
}

#[test]
fn a_tool_becomes_an_async_function_of_a_class_named_for_its_server() {
  let tool = json!({
    "name": "create_issue",
    "description": "Create a new issue.\nIt opens it on GitHub.",
    "inputSchema": {
      "type": "object",
      "properties": { "owner": { "type": "string" }, "body": { "type": "string" }, "from": { "type": "integer" } },
      "required": ["owner"]
    }
  });
  let made = binding("my-server", &[tool]);
  assert_eq!(
    made.word,
    "class my_server:\n  async def create_issue(owner: str, *, body: str | None = None, from_: int | None = None) -> McpResult:\n    \"Create a new issue.\"\n    return await mcp_call(\"my-server\", \"create_issue\", {\"owner\": owner, **{k: v for k, v in {\"body\": body, \"from\": from_}.items() if v is not None}})\n"
  );
  assert_eq!((made.name.as_str(), made.exact, made.loose.len()), ("my_server", 1, 0));
}

#[test]
fn each_type_of_json_schema_maps_to_its_type_of_python() {
  let cases = [
    (json!({ "type": "string" }), "str"),
    (json!({ "type": "integer" }), "int"),
    (json!({ "type": "number" }), "float"),
    (json!({ "type": "boolean" }), "bool"),
    (json!({ "type": "null" }), "None"),
    (json!({ "type": "array", "items": { "type": "string" } }), "list[str]"),
    (json!({ "type": "array" }), "list[Any]"),
    (json!({ "type": "object" }), "dict[str, Any]"),
    (json!({ "type": "object", "additionalProperties": { "type": "integer" } }), "dict[str, int]"),
    (json!({ "enum": ["a", "b"] }), "Literal[\"a\", \"b\"]"),
    (json!({ "enum": ["a", null] }), "Literal[\"a\"] | None"),
    (json!({ "const": 3 }), "Literal[3]"),
    (json!({ "const": true }), "Literal[True]"),
    (json!({ "type": ["string", "null"] }), "str | None"),
    (json!({ "anyOf": [{ "type": "string" }, { "type": "integer" }] }), "str | int"),
    (json!({ "oneOf": [{ "const": "a" }, { "const": "b" }] }), "Literal[\"a\", \"b\"]"),
    (json!({ "anyOf": [{ "type": "string" }, { "const": "a" }] }), "str"),
    (json!({ "type": "string", "nullable": true }), "str | None"),
    (json!({ "$ref": "#/$defs/n" }), "int"),
    (json!({ "allOf": [{ "$ref": "#/$defs/n" }] }), "int"),
    (json!({ "type": "string", "anyOf": [{ "format": "date" }, { "format": "uri" }] }), "str"),
    (json!({}), "Any"),
    (json!(true), "Any"),
  ];
  for (schema, python) in cases {
    let shown = schema.to_string();
    assert_eq!(
      mapped(schema, json!({ "n": { "type": "integer" } })),
      (python.to_owned(), Vec::new()),
      "{shown}"
    );
  }
}

#[test]
fn a_part_that_python_cannot_say_maps_to_any_and_says_why() {
  let cases = [
    (json!({ "enum": [1.5] }), "Any", "x: a Literal of Python cannot hold 1.5"),
    (
      json!({ "type": "array", "prefixItems": [{ "type": "string" }] }),
      "list[Any]",
      "x: its items differ by their place",
    ),
    (json!({ "$ref": "#/$defs/t" }), "list[Any]", "x: #/$defs/t holds itself"),
    (json!({ "$ref": "#/$defs/none" }), "Any", "x: #/$defs/none is no part of its schema"),
    (json!({ "not": { "type": "string" } }), "Any", "x: Python cannot say its not"),
    (
      json!({ "allOf": [{ "type": "string" }, { "type": "integer" }] }),
      "Any",
      "x: its allOf joins schemas of several types",
    ),
    (json!(false), "Any", "x: its schema takes no value"),
  ];
  for (schema, python, why) in cases {
    let defs = json!({ "t": { "type": "array", "items": { "$ref": "#/$defs/t" } } });
    assert_eq!(mapped(schema, defs), (python.to_owned(), vec![why.to_owned()]));
  }
}

#[test]
fn the_docstring_says_what_python_cannot() {
  let tool = json!({
    "name": "t",
    "description": "Do it. Then more.",
    "inputSchema": { "type": "object", "properties": { "x": { "enum": [1.5, 2.5] } } }
  });
  assert_eq!(
    binding("s", &[tool]).functions[0].doc,
    "Do it. Loose: x is Any, since a Literal of Python cannot hold 1.5; its schema: {\"enum\":[1.5,2.5]}."
  );
}

#[test]
fn an_object_that_names_its_properties_is_a_dict_that_maps_its_tool_loosely() {
  let input = json!({
    "type": "object",
    "properties": { "page": { "type": "object", "properties": { "id": { "type": "string" } } } }
  });
  let made = binding("s", &[json!({ "name": "t", "inputSchema": input })]);
  assert_eq!(made.functions[0].params[0].shape, Shape::Dict(Box::new(Shape::Any)));
  assert_eq!(
    (made.exact, made.loose.clone()),
    (0, vec!["t: page: an object whose properties Python cannot type".to_owned()])
  );
}

#[test]
fn the_docstring_sketches_each_object_whose_properties_python_cannot_type() {
  let input = json!({
    "type": "object",
    "properties": {
      "items": { "type": "array", "items": { "$ref": "#/$defs/item" } }
    },
    "required": ["items"],
    "$defs": { "item": { "type": "object", "properties": { "id": { "type": "string" }, "n": { "type": "integer" } }, "required": ["id"] } }
  });
  let made =
    binding("s", &[json!({ "name": "t", "description": "Add items.", "inputSchema": input })]);
  assert_eq!(made.functions[0].doc, "Add items. Dicts: items is list[{\"id\": str, \"n\"?: int}].");
}

#[test]
fn the_gate_refuses_a_call_that_its_signature_does_not_take() {
  let input = json!({ "type": "object", "properties": { "a": { "type": "string" }, "b": { "enum": ["x", "y"] } }, "required": ["a"] });
  let made = binding("s", &[json!({ "name": "t", "inputSchema": input })]);
  let wrong = "await s.t(1)\nawait s.t('a', b='z')\nawait s.t('a', c=1)\nawait s.u()";
  let findings: Vec<String> = found(&[WORD, &made.word], wrong)
    .into_iter()
    .map(|one| one.split(' ').take(3).collect::<Vec<_>>().join(" "))
    .collect();
  assert_eq!(
    findings,
    [
      "line 1: error[invalid-argument-type]",
      "line 2: error[invalid-argument-type]",
      "line 3: error[unknown-argument]",
      "line 4: error[unresolved-attribute]",
    ]
  );
}

#[test]
fn each_name_becomes_safe_and_unique() {
  assert_eq!(
    [
      safe("from", "x", false),
      safe("x-y", "x", false),
      safe("1st", "x", false),
      safe("__init__", "x", false)
    ],
    ["from_", "x_y", "_1st", "_init__"]
  );
  assert_eq!(
    [safe("", "argument", false), safe("list", "x", true), safe("list", "x", false)],
    ["argument", "list_", "list"]
  );
  let tools = [json!({ "name": "a-b" }), json!({ "name": "a_b" }), json!({ "name": "str" })];
  let names: Vec<String> =
    binding("for", &tools).functions.iter().map(|one| one.name.clone()).collect();
  assert_eq!(names, ["a_b", "a_b_", "str_"]);
  assert!(binding("for", &tools).word.starts_with("class for_:\n"));
}

#[test]
fn a_call_sends_each_required_argument_and_each_optional_one_that_is_not_none_under_its_original_name()
 {
  let input = json!({
    "type": "object",
    "properties": {
      "to-x": { "type": "string" },
      "from": { "type": ["integer", "null"] },
      "k": { "type": "integer" },
      "mcp_call": { "type": "boolean" }
    },
    "required": ["from"]
  });
  let made = binding("srv", &[json!({ "name": "go", "inputSchema": input })]);
  assert_eq!(
    made.functions[0]
      .params
      .iter()
      .map(|one| (one.name.as_str(), one.required))
      .collect::<Vec<_>>(),
    [("from_", true), ("to_x", false), ("k", false), ("mcp_call_", false)]
  );
  let calls = [
    "await srv.go(None)".to_owned(),
    "await srv.go(from_=2, to_x='a', k=None, mcp_call_=False)".to_owned(),
  ];
  assert_eq!(
    sent(&made.word, &calls),
    [
      json!({ "server": "srv", "tool": "go", "arguments": { "from": null } }),
      json!({ "server": "srv", "tool": "go", "arguments": { "from": 2, "to-x": "a", "mcp_call": false } }),
    ]
  );
}

#[test]
fn a_tool_with_no_arguments_takes_none_and_sends_an_empty_dict() {
  let made = binding(
    "s",
    &[json!({ "name": "ping", "inputSchema": { "type": "object" } }), json!({ "name": "pong" })],
  );
  assert!(made.word.contains(
    "  async def ping() -> McpResult:\n    return await mcp_call(\"s\", \"ping\", {})\n"
  ));
  assert_eq!(
    sent(&made.word, &["await s.pong()".to_owned()]),
    [json!({ "server": "s", "tool": "pong", "arguments": {} })]
  );
}

#[test]
fn the_arguments_that_a_schema_takes_and_names_nowhere_are_keywords_of_their_own() {
  let input = json!({ "type": "object", "properties": { "a": { "type": "string" } }, "required": ["a"], "additionalProperties": { "type": "integer" } });
  let made = binding("s", &[json!({ "name": "t", "inputSchema": input })]);
  assert!(made.word.contains("async def t(a: str, **arguments: int) -> McpResult:"));
  assert_eq!(
    sent(&made.word, &["await s.t('x', b=1)".to_owned()]),
    [json!({ "server": "s", "tool": "t", "arguments": { "a": "x", "b": 1 } })]
  );
}

#[test]
fn the_forms_of_the_arguments_of_a_tool_are_optional_and_loose() {
  let input = json!({
    "type": "object",
    "oneOf": [
      { "properties": { "a": { "type": "string" } }, "required": ["a"] },
      { "properties": { "b": { "type": "integer" } }, "required": ["b"] }
    ]
  });
  let made = binding("s", &[json!({ "name": "t", "inputSchema": input })]);
  assert!(
    made.word.contains("async def t(*, a: str | None = None, b: int | None = None) -> McpResult:")
  );
  assert_eq!(
    made.loose,
    ["t: its arguments take one of 2 forms, and each argument that a form alone names is optional"]
  );
}

#[test]
fn a_server_with_no_tool_binds_nothing() {
  assert_eq!(binding("s", &[]).word, "");
}

#[test]
fn a_union_is_flat_with_each_shape_once_and_none_last() {
  let shapes = vec![
    Shape::None,
    Shape::Union(vec![Shape::Str, Shape::Literal(vec![json!(1)])]),
    Shape::Literal(vec![json!("a"), json!(2)]),
    Shape::Str,
  ];
  assert_eq!(union(shapes).python(), "str | Literal[1, 2] | None");
  assert_eq!(union(vec![Shape::Int, Shape::Any]), Shape::Any);
}

#[test]
fn each_word_of_the_corpus_and_each_call_of_its_functions_pass_the_gate() {
  for (server, file) in corpus() {
    let made = binding(&server, tools(&file));
    assert_eq!(found(&[WORD], &made.word), Vec::<String>::new(), "{server}");
    let made_calls: Vec<String> = made
      .functions
      .iter()
      .flat_map(|one| calls(&server, &made.name, one))
      .map(|(call, _)| call)
      .collect();
    assert_eq!(
      found(&[WORD, &made.word], &made_calls.join("\n")),
      Vec::<String>::new(),
      "{server}"
    );
  }
}

#[test]
fn each_call_of_the_corpus_sends_in_monty_what_its_signature_gives() {
  for (server, file) in corpus() {
    let made = binding(&server, tools(&file));
    let (made_calls, wanted): (Vec<String>, Vec<Value>) =
      made.functions.iter().flat_map(|one| calls(&server, &made.name, one)).unzip();
    assert_eq!(sent(&made.word, &made_calls), wanted, "{server}");
  }
}

#[test]
fn each_call_of_a_function_that_maps_its_tool_exactly_sends_what_its_schema_asks() {
  let report = std::env::var("FURB_MCP_MEASURE").ok();
  let mut lines = vec![
    "| server | tools | exact | opaque objects | loose | word chars | calls | valid | constraint only | shape |".to_owned(),
    "|---|---|---|---|---|---|---|---|---|---|".to_owned(),
  ];
  let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
  let mut constraints: BTreeMap<String, usize> = BTreeMap::new();
  let mut wrong = Vec::new();
  for (server, file) in corpus() {
    let listed = tools(&file);
    let made = binding(&server, listed);
    if let Some(path) = &report {
      fs::create_dir_all(format!("{path}.words")).expect("the folder of the words stands");
      fs::write(format!("{path}.words/{server}.py"), &made.word).expect("the word writes");
      fs::write(format!("{path}.words/{server}.calls"), "").expect("the calls write");
    }
    let mut counts = [0usize; 4];
    for (one, tool) in made.functions.iter().zip(listed) {
      let schema = tool.get("inputSchema").cloned().unwrap_or(json!(true));
      let validator = jsonschema::validator_for(&schema)
        .unwrap_or_else(|fault| panic!("the schema of {server} {} compiles: {fault}", one.tool));
      for (call, sends) in calls(&server, &made.name, one) {
        let keywords: Vec<String> = validator
          .iter_errors(&sends["arguments"])
          .map(|fault| {
            fault.schema_path.to_string().rsplit('/').next().unwrap_or_default().to_owned()
          })
          .collect();
        let shaped = keywords.iter().any(|keyword| SHAPED.contains(&keyword.as_str()));
        counts[0] += 1;
        counts[if keywords.is_empty() {
          1
        } else if shaped {
          3
        } else {
          2
        }] += 1;
        if !shaped {
          for keyword in &keywords {
            *constraints.entry(keyword.clone()).or_default() += 1;
          }
        }
        if let Some(path) = &report {
          fs::create_dir_all(format!("{path}.words")).expect("the folder of the words stands");
          let line = format!("{}\t{}\n", call, sends);
          let file = format!("{path}.words/{server}.calls");
          let held = fs::read_to_string(&file).unwrap_or_default();
          fs::write(&file, held + &line).expect("the calls write");
        }
        if !keywords.is_empty() && one.exact() {
          wrong.push(format!("{server} {call}: {keywords:?}"));
        }
      }
      for why in one.loose.iter().chain(one.opaque.iter()) {
        let kind = why.split_once(": ").map_or(why.as_str(), |(_, kind)| kind);
        let kind: String =
          kind.chars().take_while(|one| !one.is_ascii_digit() && *one != '#').collect();
        *reasons
          .entry(if one.opaque.contains(why) {
            "an object whose properties Python cannot type".to_owned()
          } else {
            kind
          })
          .or_default() += 1;
      }
    }
    let opaque =
      made.functions.iter().filter(|one| one.loose.is_empty() && !one.opaque.is_empty()).count();
    let loose = made.functions.iter().filter(|one| !one.loose.is_empty()).count();
    lines.push(format!(
      "| {server} | {} | {} | {opaque} | {loose} | {} | {} | {} | {} | {} |",
      made.functions.len(),
      made.exact,
      made.word.chars().count(),
      counts[0],
      counts[1],
      counts[2],
      counts[3]
    ));
  }
  if let Some(path) = report {
    let kinds: Vec<String> = reasons.iter().map(|(why, many)| format!("- {many}: {why}")).collect();
    let kept: Vec<String> =
      constraints.iter().map(|(keyword, many)| format!("- {many}: {keyword}")).collect();
    let text = format!(
      "{}\n\nWhy a tool maps loosely, by kind:\n{}\n\nThe keywords that a call of a right shape fails, by count:\n{}\n",
      lines.join("\n"),
      kinds.join("\n"),
      kept.join("\n")
    );
    fs::write(path, text).expect("the measure writes");
  }
  assert_eq!(wrong, Vec::<String>::new());
}
