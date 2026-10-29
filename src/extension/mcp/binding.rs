//! The binding of the tools of a server of MCP: a word of Python made from their schemas, which a chain runs so that
//! each tool is a function that the gate checks.
//!
//! The word is one class under the name of the server, with one async function for each tool, which takes no self and
//! has no decorator: the sandbox binds no `staticmethod`, and a function of a class reads in the gate and runs on both
//! engines as a plain function, so a model calls `await github.create_issue(owner="o", repo="r", title="t")`. The
//! name of the class stands in the word on its first line alone, so a word that finds the name taken renames the class
//! with one replace. The word imports nothing: it names `Any` and `Literal`, which the module of each chain holds from
//! the engine, and `McpResult` and `mcp_call`, which the word of the extension binds. `McpResult` is
//! `dict[str, Any]`, since monty cannot import `TypedDict`.

use serde_json::{Map, Value};

/// The most characters of the first sentence of a description that a docstring holds.
const SENTENCE: usize = 200;

/// The most characters of the schema of a parameter that a docstring quotes when Python cannot say what it asks.
const QUOTED: usize = 160;

/// The most characters of the sketch of an object that a docstring holds.
const SKETCHED: usize = 300;

/// The most levels of objects and lists, one in another, that the sketch of an object shows.
const DEPTH: usize = 4;

/// The most references that the schema of the arguments of a tool follows at its top, one to the next.
const FOLLOWED: usize = 32;

/// The keywords of Python, which no name of a word takes.
const KEYWORDS: [&str; 35] = [
  "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
  "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in",
  "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with",
  "yield",
];

/// The names that the annotations of a function read in the body of its class, which no class and no function of a
/// class takes: a function of that name would stand in place of the type in each later annotation.
const TYPES: [&str; 9] =
  ["Any", "Literal", "McpResult", "str", "int", "float", "bool", "list", "dict"];

/// The name that the body of each function reads, which no parameter takes.
const CALL: &str = "mcp_call";

/// The schema that takes any value, which a tool with no schema of its arguments has.
static ANY: Value = Value::Bool(true);

/// The type of a value in Python, as a binding maps it from a schema.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
  /// Any value: the schema takes any, or Python cannot say what it asks.
  Any,
  /// A str.
  Str,
  /// An int.
  Int,
  /// A float, which an int is too.
  Float,
  /// A bool.
  Bool,
  /// None, which sends null.
  None,
  /// A list of values of one shape.
  List(Box<Shape>),
  /// A dict of keys of str and values of one shape.
  Dict(Box<Shape>),
  /// One of these values, each a str, an int or a bool.
  Literal(Vec<Value>),
  /// One of these shapes, each once, with no union and no `Any` among them, and None last.
  Union(Vec<Shape>),
}

impl Shape {
  /// The shape as an annotation of Python.
  pub fn python(&self) -> String {
    match self {
      Shape::Any => "Any".to_owned(),
      Shape::Str => "str".to_owned(),
      Shape::Int => "int".to_owned(),
      Shape::Float => "float".to_owned(),
      Shape::Bool => "bool".to_owned(),
      Shape::None => "None".to_owned(),
      Shape::List(item) => format!("list[{}]", item.python()),
      Shape::Dict(value) => format!("dict[str, {}]", value.python()),
      Shape::Literal(values) => {
        format!("Literal[{}]", values.iter().map(literal).collect::<Vec<_>>().join(", "))
      }
      Shape::Union(members) => members.iter().map(Shape::python).collect::<Vec<_>>().join(" | "),
    }
  }
}

/// A parameter of a function: its name in Python, its key in the arguments of the tool, its shape, and whether the
/// tool requires it. A call sends a required parameter as it is given, and an optional one only when it is not None.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
  pub name: String,
  pub key: String,
  pub shape: Shape,
  pub required: bool,
}

/// The function of one tool: the name of the tool, the name of the function, its docstring, its parameters with the
/// required ones first, the name and the shape of the arguments that the schema takes and names nowhere, which the
/// function takes as `**name`, why each part maps loosely, and the key of each parameter that is an object whose
/// properties Python cannot type.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
  pub tool: String,
  pub name: String,
  pub doc: String,
  pub params: Vec<Param>,
  pub rest: Option<(String, Shape)>,
  pub loose: Vec<String>,
  pub opaque: Vec<String>,
}

impl Function {
  /// Whether the function maps its tool exactly: no part maps loosely, and no parameter is an object whose
  /// properties Python cannot type.
  pub fn exact(&self) -> bool {
    self.loose.is_empty() && self.opaque.is_empty()
  }

  /// The function as Python, in the body of the class of its server.
  fn python(&self, server: &str) -> String {
    let mut names: Vec<String> = self.params.iter().map(|one| one.name.clone()).collect();
    names.extend(self.rest.iter().map(|(name, _)| name.clone()));
    let (key, value) = (unique("k".to_owned(), &mut names), unique("v".to_owned(), &mut names));
    let (required, optional): (Vec<&Param>, Vec<&Param>) =
      self.params.iter().partition(|one| one.required);
    let mut signature: Vec<String> =
      required.iter().map(|one| format!("{}: {}", one.name, one.shape.python())).collect();
    if !optional.is_empty() {
      signature.push("*".to_owned());
      signature.extend(optional.iter().map(|one| {
        format!("{}: {} = None", one.name, union(vec![one.shape.clone(), Shape::None]).python())
      }));
    }
    signature.extend(self.rest.iter().map(|(name, shape)| format!("**{name}: {}", shape.python())));
    let pair = |one: &&Param| format!("{}: {}", quoted(&one.key), one.name);
    let mut sent: Vec<String> = required.iter().map(pair).collect();
    if !optional.is_empty() {
      let given = optional.iter().map(pair).collect::<Vec<_>>().join(", ");
      sent.push(format!(
        "**{{{key}: {value} for {key}, {value} in {{{given}}}.items() if {value} is not None}}"
      ));
    }
    sent.extend(self.rest.iter().map(|(name, _)| format!("**{name}")));
    let arguments = match sent.as_slice() {
      [one] if one.starts_with("**") => one[2..].to_owned(),
      _ => format!("{{{}}}", sent.join(", ")),
    };
    let doc =
      if self.doc.is_empty() { String::new() } else { format!("    {}\n", quoted(&self.doc)) };
    format!(
      "  async def {}({}) -> McpResult:\n{doc}    return await {CALL}({}, {}, {arguments})\n",
      self.name,
      signature.join(", "),
      quoted(server),
      quoted(&self.tool),
    )
  }
}

/// What a binding made: its word, the name of its class, how many tools it maps exactly, why each other tool does
/// not, and the function of each tool.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
  pub word: String,
  pub name: String,
  pub exact: usize,
  pub loose: Vec<String>,
  pub functions: Vec<Function>,
}

/// The binding of the tools of a server, each as MCP lists it: its name, its description, and the schema of its
/// arguments under `inputSchema`, or under `input` as the ear gives it. A server with no tool binds nothing, and its
/// word is empty. Each name that is no identifier of Python, or is a keyword, becomes safe, as `from` becomes `from_`,
/// and a call sends the original name.
pub fn binding(server: &str, tools: &[Value]) -> Binding {
  let name = safe(server, "server", true);
  let mut taken = Vec::new();
  let functions: Vec<Function> = tools
    .iter()
    .filter_map(|tool| Some(function(tool.get("name")?.as_str()?, tool, &mut taken)))
    .collect();
  let exact = functions.iter().filter(|one| one.exact()).count();
  let loose = functions
    .iter()
    .filter(|one| !one.exact())
    .map(|one| {
      let objects = one
        .opaque
        .iter()
        .map(|key| format!("{key}: an object whose properties Python cannot type"));
      format!(
        "{}: {}",
        one.tool,
        one.loose.iter().cloned().chain(objects).collect::<Vec<_>>().join("; ")
      )
    })
    .collect();
  let word = if functions.is_empty() {
    String::new()
  } else {
    functions.iter().fold(format!("class {name}:\n"), |word, one| word + &one.python(server))
  };
  Binding { word, name, exact, loose, functions }
}

/// The function of one tool, whose name the functions before it did not take.
fn function(tool: &str, about: &Value, taken: &mut Vec<String>) -> Function {
  let name = unique(safe(tool, "tool", true), taken);
  let input = about.get("inputSchema").or_else(|| about.get("input")).unwrap_or(&ANY);
  let mut mapper = Mapper { root: input, seen: Vec::new(), notes: Vec::new(), opaque: 0 };
  let (fields, rest) = mapper.fields();
  let mut loose = std::mem::take(&mut mapper.notes);
  let mut said = loose.clone();
  let mut names = vec![CALL.to_owned()];
  let mut params = Vec::new();
  let mut opaque = Vec::new();
  let mut sketches = Vec::new();
  for (key, schema, required) in fields {
    let objects = mapper.opaque;
    let shape = mapper.shape(&schema);
    let reasons = std::mem::take(&mut mapper.notes);
    let name = unique(safe(&key, "argument", false), &mut names);
    if !reasons.is_empty() {
      loose.push(format!("{key}: {}", reasons.join(", ")));
      let asks = cut(&schema.to_string(), QUOTED);
      said.push(format!(
        "{name} is {}, since {}; its schema: {asks}",
        shape.python(),
        reasons.join(", ")
      ));
    }
    if mapper.opaque > objects {
      opaque.push(key.clone());
      sketches.push(format!("{name} is {}", cut(&mapper.sketch(&schema, DEPTH), SKETCHED)));
    }
    params.push(Param { name, key, shape, required });
  }
  params.sort_by_key(|one| !one.required);
  let rest = rest.map(|shape| (unique("arguments".to_owned(), &mut names), shape));
  let described = ["/description", "/title", "/annotations/title"]
    .iter()
    .find_map(|at| about.pointer(at).and_then(Value::as_str).filter(|text| !text.trim().is_empty()))
    .map(sentence)
    .unwrap_or_default();
  let mut doc = described;
  if !doc.is_empty() && !doc.ends_with(['.', '!', '?']) {
    doc.push('.');
  }
  if !sketches.is_empty() {
    doc = format!("{doc} Dicts: {}.", sketches.join("; "));
  }
  if !said.is_empty() {
    doc = format!("{doc} Loose: {}.", said.join("; "));
  }
  let doc = doc.trim_start().to_owned();
  Function { tool: tool.to_owned(), name, doc, params, rest, loose, opaque }
}

/// What maps the schemas of one tool: the schema of its arguments, which each reference resolves in, the references
/// that the shape now stands in, why each part mapped loosely, and how many objects whose properties Python cannot
/// type it mapped.
struct Mapper<'a> {
  root: &'a Value,
  seen: Vec<String>,
  notes: Vec<String>,
  opaque: usize,
}

impl<'a> Mapper<'a> {
  /// A part that Python cannot say, with why, which maps to `Any`.
  fn note(&mut self, why: String) -> Shape {
    self.notes.push(why);
    Shape::Any
  }

  /// The schema that a schema refers to at its top, as far as references go, or nothing when one refers to a part
  /// that the root does not hold, or they never end.
  fn followed(&self, mut at: &'a Value) -> Option<&'a Value> {
    let root = self.root;
    for _ in 0..FOLLOWED {
      let Some(target) = at.get("$ref").and_then(Value::as_str) else { return Some(at) };
      at = target.strip_prefix('#').and_then(|path| root.pointer(path))?;
    }
    None
  }

  /// The arguments of the tool, each as its key, its schema and whether the tool requires it, and the shape of the
  /// arguments that the schema takes and names nowhere, when it takes them.
  fn fields(&mut self) -> (Vec<(String, Value, bool)>, Option<Shape>) {
    let Some(object) = self.followed(self.root) else {
      self
        .notes
        .push("the schema of its arguments refers to a part that it does not hold".to_owned());
      return (Vec::new(), Some(Shape::Any));
    };
    let one = match object {
      Value::Object(one) => one,
      Value::Bool(true) => return (Vec::new(), None),
      other => {
        self
          .notes
          .push(format!("the schema of its arguments is {}", cut(&other.to_string(), QUOTED)));
        return (Vec::new(), Some(Shape::Any));
      }
    };
    if let Some(kind) = one.get("type")
      && kind.as_str() != Some("object")
      && !kind
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|one| one.as_str() == Some("object")))
    {
      self.notes.push(format!("its arguments are of the type {kind}, and not an object"));
      return (Vec::new(), Some(Shape::Any));
    }
    let mut fields = Vec::new();
    let mut required: Vec<String> = Vec::new();
    let mut parts = vec![one];
    if let Some(Value::Array(members)) = one.get("allOf") {
      parts.extend(members.iter().filter_map(|member| self.followed(member)?.as_object()));
    }
    for part in &parts {
      listed(part, &mut fields);
      required.extend(
        part
          .get("required")
          .and_then(Value::as_array)
          .into_iter()
          .flatten()
          .filter_map(|key| key.as_str().map(str::to_owned)),
      );
    }
    for key in ["anyOf", "oneOf"] {
      if let Some(Value::Array(members)) = one.get(key) {
        let before = fields.len();
        for member in members.iter().filter_map(|member| self.followed(member)?.as_object()) {
          listed(member, &mut fields);
        }
        if fields.len() > before {
          self.notes.push(format!(
            "its arguments take one of {} forms, and each argument that a form alone names is optional",
            members.len()
          ));
        }
      }
    }
    for key in required {
      match fields.iter_mut().find(|field| field.0 == key) {
        Some(field) => field.2 = true,
        None => {
          self.notes.push(format!("{key} is required and has no schema"));
          fields.push((key, ANY.clone(), true));
        }
      }
    }
    let rest = match one.get("additionalProperties") {
      Some(Value::Bool(true)) => Some(Shape::Any),
      Some(schema @ Value::Object(_)) => Some(self.shape(schema)),
      _ if one.contains_key("patternProperties") => Some(Shape::Any),
      _ => None,
    };
    (fields, rest)
  }

  /// The sketch of a schema for a docstring: an object that names its properties shows each, as `"key": shape`, with
  /// `?` after the key of a property that it does not require, and a list shows its items, to `depth` levels.
  fn sketch(&mut self, schema: &Value, depth: usize) -> String {
    let at = self.followed(schema).unwrap_or(schema);
    let Some(one) = at.as_object() else { return "Any".to_owned() };
    if let Some(named) =
      one.get("properties").and_then(Value::as_object).filter(|named| !named.is_empty())
    {
      if depth == 0 {
        return "dict".to_owned();
      }
      let required: Vec<&str> = one
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
      let parts: Vec<String> = named
        .iter()
        .map(|(key, part)| {
          let optional = if required.contains(&key.as_str()) { "" } else { "?" };
          format!("{}{optional}: {}", quoted(key), self.sketch(part, depth - 1))
        })
        .collect();
      return format!("{{{}}}", parts.join(", "));
    }
    for key in ["anyOf", "oneOf"] {
      if let Some(Value::Array(members)) = one.get(key)
        && members.iter().any(typed)
      {
        if depth == 0 {
          return "dict".to_owned();
        }
        let mut kept: Vec<String> = Vec::new();
        for member in members {
          let part = self.sketch(member, depth - 1);
          if !kept.contains(&part) {
            kept.push(part);
          }
        }
        return kept.join(" | ");
      }
    }
    let listed =
      one.get("type").and_then(Value::as_str) == Some("array") || one.contains_key("items");
    if listed && let Some(items @ Value::Object(_)) = one.get("items") {
      return if depth == 0 {
        "list".to_owned()
      } else {
        format!("list[{}]", self.sketch(items, depth - 1))
      };
    }
    let (notes, opaque) = (self.notes.len(), self.opaque);
    let got = self.shape(at).python();
    self.notes.truncate(notes);
    self.opaque = opaque;
    got
  }

  /// The shape of a schema.
  fn shape(&mut self, schema: &Value) -> Shape {
    let Some(one) = schema.as_object() else {
      return match schema {
        Value::Bool(true) => Shape::Any,
        Value::Bool(false) => self.note("its schema takes no value".to_owned()),
        other => self.note(format!("its schema {} is no schema", cut(&other.to_string(), QUOTED))),
      };
    };
    let got = self.core(one);
    if one.get("nullable") == Some(&Value::Bool(true)) {
      union(vec![got, Shape::None])
    } else {
      got
    }
  }

  /// The shape of a schema that is an object, before `nullable`.
  fn core(&mut self, one: &Map<String, Value>) -> Shape {
    if let Some(at) = one.get("$ref").and_then(Value::as_str) {
      if self.seen.iter().any(|seen| seen == at) {
        return self.note(format!("{at} holds itself"));
      }
      let root = self.root;
      let Some(found) = at.strip_prefix('#').and_then(|path| root.pointer(path)) else {
        return self.note(format!("{at} is no part of its schema"));
      };
      self.seen.push(at.to_owned());
      let got = self.shape(found);
      self.seen.pop();
      return got;
    }
    if let Some(value) = one.get("const") {
      return self.literal(std::slice::from_ref(value));
    }
    if let Some(values) = one.get("enum") {
      return match values.as_array() {
        Some(values) if !values.is_empty() => self.literal(values),
        _ => self.note("its enum holds no value".to_owned()),
      };
    }
    for key in ["anyOf", "oneOf"] {
      if let Some(Value::Array(members)) = one.get(key)
        && members.iter().any(typed)
      {
        let parts = members.iter().map(|member| self.shape(member)).collect();
        return union(parts);
      }
    }
    if let Some(Value::Array(members)) = one.get("allOf") {
      let members: Vec<&Value> = members.iter().filter(|member| typed(member)).collect();
      if !members.is_empty() {
        let shapes: Vec<Shape> = members.into_iter().map(|member| self.shape(member)).collect();
        if shapes.iter().all(|shape| matches!(shape, Shape::Dict(_))) {
          return Shape::Dict(Box::new(Shape::Any));
        }
        if shapes.windows(2).all(|pair| pair[0] == pair[1]) {
          return shapes[0].clone();
        }
        return self.note("its allOf joins schemas of several types".to_owned());
      }
    }
    match one.get("type") {
      Some(Value::String(kind)) => self.kind(kind, one),
      Some(Value::Array(kinds)) => {
        let parts = kinds
          .iter()
          .map(|kind| match kind.as_str() {
            Some(kind) => self.kind(kind, one),
            None => self.note(format!("its type {kind} is no type of JSON")),
          })
          .collect();
        union(parts)
      }
      Some(other) => self.note(format!("its type {other} is no type of JSON")),
      None
        if ["properties", "additionalProperties", "patternProperties"]
          .iter()
          .any(|key| one.contains_key(*key)) =>
      {
        self.object(one)
      }
      None if one.contains_key("items") || one.contains_key("prefixItems") => {
        self.kind("array", one)
      }
      None => match ["not", "if"].iter().find(|key| one.contains_key(**key)) {
        Some(key) => self.note(format!("Python cannot say its {key}")),
        None => Shape::Any,
      },
    }
  }

  /// The shape of one type of JSON, with the rest of its schema.
  fn kind(&mut self, kind: &str, one: &Map<String, Value>) -> Shape {
    match kind {
      "string" => Shape::Str,
      "integer" => Shape::Int,
      "number" => Shape::Float,
      "boolean" => Shape::Bool,
      "null" => Shape::None,
      "array" => {
        let item =
          if one.contains_key("prefixItems") || matches!(one.get("items"), Some(Value::Array(_))) {
            self.note("its items differ by their place".to_owned())
          } else {
            match one.get("items") {
              Some(items) => self.shape(items),
              None => Shape::Any,
            }
          };
        Shape::List(Box::new(item))
      }
      "object" => self.object(one),
      other => self.note(format!("its type {other:?} is no type of JSON")),
    }
  }

  /// The shape of an object: a dict whose values have the shape of the schema of its additional properties, or any
  /// value when it names its properties, which Python cannot type in the sandbox.
  fn object(&mut self, one: &Map<String, Value>) -> Shape {
    let named =
      one.get("properties").and_then(Value::as_object).is_some_and(|named| !named.is_empty());
    if named || one.contains_key("patternProperties") {
      self.opaque += 1;
      return Shape::Dict(Box::new(Shape::Any));
    }
    match one.get("additionalProperties") {
      Some(schema @ Value::Object(_)) => Shape::Dict(Box::new(self.shape(schema))),
      _ => Shape::Dict(Box::new(Shape::Any)),
    }
  }

  /// The shape of the values of an enum or a const: a Literal of each str, int and bool, and None for null.
  fn literal(&mut self, values: &[Value]) -> Shape {
    let mut kept = Vec::new();
    let mut none = false;
    for value in values {
      match value {
        Value::Null => none = true,
        Value::String(_) | Value::Bool(_) => kept.push(value.clone()),
        Value::Number(number) if number.is_i64() || number.is_u64() => kept.push(value.clone()),
        other => {
          return self
            .note(format!("a Literal of Python cannot hold {}", cut(&other.to_string(), QUOTED)));
        }
      }
    }
    let mut parts = Vec::new();
    if !kept.is_empty() {
      parts.push(Shape::Literal(kept));
    }
    if none {
      parts.push(Shape::None);
    }
    union(parts)
  }
}

/// Each property of a schema that the fields do not hold yet, as optional.
fn listed(part: &Map<String, Value>, fields: &mut Vec<(String, Value, bool)>) {
  for (key, schema) in part.get("properties").and_then(Value::as_object).into_iter().flatten() {
    if !fields.iter().any(|field| field.0 == *key) {
      fields.push((key.clone(), schema.clone(), false));
    }
  }
}

/// Whether a member of a union says a type, and is no mere constraint on the type of its parent.
fn typed(member: &Value) -> bool {
  match member {
    Value::Object(one) => {
      ["type", "enum", "const", "$ref", "anyOf", "oneOf", "allOf", "properties", "items"]
        .iter()
        .any(|key| one.contains_key(*key))
    }
    _ => true,
  }
}

/// The union of shapes: flat, each shape once, the values of each Literal in one Literal less those that a plain type
/// of the union takes, None last, and `Any` when a member is `Any` or no member stands.
pub fn union(parts: Vec<Shape>) -> Shape {
  let mut kept: Vec<Shape> = Vec::new();
  let mut values: Vec<Value> = Vec::new();
  let mut todo = parts;
  todo.reverse();
  while let Some(one) = todo.pop() {
    match one {
      Shape::Any => return Shape::Any,
      Shape::Union(members) => todo.extend(members.into_iter().rev()),
      Shape::Literal(given) => {
        if !kept.iter().any(|shape| matches!(shape, Shape::Literal(_))) {
          kept.push(Shape::Literal(Vec::new()));
        }
        for value in given {
          if !values.contains(&value) {
            values.push(value);
          }
        }
      }
      other => {
        if !kept.contains(&other) {
          kept.push(other);
        }
      }
    }
  }
  values.retain(|value| !match value {
    Value::String(_) => kept.contains(&Shape::Str),
    Value::Bool(_) => kept.contains(&Shape::Bool),
    _ => kept.contains(&Shape::Int) || kept.contains(&Shape::Float),
  });
  let mut shapes: Vec<Shape> = kept
    .into_iter()
    .filter_map(|shape| match shape {
      Shape::Literal(_) if values.is_empty() => None,
      Shape::Literal(_) => Some(Shape::Literal(values.clone())),
      other => Some(other),
    })
    .collect();
  if let Some(at) = shapes.iter().position(|shape| *shape == Shape::None) {
    let none = shapes.remove(at);
    shapes.push(none);
  }
  match shapes.len() {
    0 => Shape::Any,
    1 => shapes.remove(0),
    _ => Shape::Union(shapes),
  }
}

/// A name made safe for Python: each character that is no letter, digit or underscore of ASCII becomes an underscore,
/// a name with two underscores or more at its start keeps one, since a class mangles such a name, a name that starts
/// with a digit takes an underscore before it, a keyword takes one after it, and an empty name is `empty`. A name of a
/// class or of a function of a class takes an underscore after it too when an annotation reads it as a type.
pub fn safe(name: &str, empty: &str, typing: bool) -> String {
  let mut made: String = name
    .chars()
    .map(|one| if one.is_ascii_alphanumeric() || one == '_' { one } else { '_' })
    .collect();
  if made.starts_with("__") {
    made = format!("_{}", made.trim_start_matches('_'));
  }
  if made.is_empty() {
    made = empty.to_owned();
  }
  if made.starts_with(|one: char| one.is_ascii_digit()) {
    made.insert(0, '_');
  }
  if KEYWORDS.contains(&made.as_str()) || (typing && TYPES.contains(&made.as_str())) {
    made.push('_');
  }
  made
}

/// A name that no name taken before takes, with underscores after it as it needs, which it then takes.
fn unique(mut name: String, taken: &mut Vec<String>) -> String {
  while taken.contains(&name) {
    name.push('_');
  }
  taken.push(name.clone());
  name
}

/// A text as a string of Python on one line.
pub fn quoted(text: &str) -> String {
  let mut made = String::from('"');
  for one in text.chars() {
    match one {
      '\\' => made.push_str("\\\\"),
      '"' => made.push_str("\\\""),
      '\n' => made.push_str("\\n"),
      '\r' => made.push_str("\\r"),
      '\t' => made.push_str("\\t"),
      _ if one.is_control() || matches!(one, '\u{2028}' | '\u{2029}' | '\u{feff}') => {
        made.push_str(&format!("\\u{:04x}", u32::from(one)));
      }
      _ => made.push(one),
    }
  }
  made.push('"');
  made
}

/// A value of an enum or a const in a Literal of Python.
fn literal(value: &Value) -> String {
  match value {
    Value::String(text) => quoted(text),
    Value::Bool(true) => "True".to_owned(),
    Value::Bool(false) => "False".to_owned(),
    other => other.to_string(),
  }
}

/// The first sentence of the first paragraph of a description, on one line, cut to `SENTENCE` characters, which a
/// docstring ends with a full stop when it has none.
fn sentence(text: &str) -> String {
  let first = text.trim().split("\n\n").next().unwrap_or_default();
  let flat = first.split_whitespace().collect::<Vec<_>>().join(" ");
  let end = flat
    .match_indices(['.', '!', '?'])
    .map(|(at, _)| at + 1)
    .find(|at| flat[*at..].starts_with(' '))
    .unwrap_or(flat.len());
  cut(&flat[..end], SENTENCE)
}

/// A text cut to its first `most` characters, which ends with three dots when it was cut.
fn cut(text: &str, most: usize) -> String {
  if text.chars().count() <= most {
    return text.to_owned();
  }
  let kept: String = text.chars().take(most.saturating_sub(3)).collect();
  format!("{}...", kept.trim_end())
}

#[cfg(test)]
#[path = "binding.test.rs"]
mod test;
