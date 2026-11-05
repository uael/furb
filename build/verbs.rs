//! The build: the contract is read here, and every verb of it becomes a method of the engine and of
//! each door.
//!
//! A verb is a function that `src/furb/engine.pyi` gives before its classes, which is what a model
//! or an operator says; the functions after them are the engine's own. A verb that the contract
//! gives only as overloads is the last of them, the widest, with the docstring of the first.
//!
//! The method of a verb takes the parameters that have no default, in their order, then a list for
//! its star parameter, then one struct of the rest, named after the verb, whose fields are none
//! until a host gives them, so the engine keeps its own defaults. A parameter with a default that
//! stands before the star parameter goes to the engine by its place, with the default of the
//! contract when the host gives none, so the words of the star follow it. The door to TypeScript
//! takes the same, with an object of options for the struct, and the door to python takes each of
//! those words by its name, but a word that stands before the star there too. A word whose type
//! holds None beside a default of its own, as the stdin of bash, takes None as a word apart from
//! that default: its field holds an option of an option, TypeScript gives it as null, and python as
//! None, and a word left out leaves the default of the engine. What each annotation is, in rust and
//! in TypeScript, is the table of [`Kind::of`] and [`Answer::of`], and an annotation that the
//! tables do not hold stops the build, so no verb of the contract goes without its method.
//!
//! Each constant of the contract that is a number or a text becomes a constant of the engine, with
//! its sentence, so no ear of the crate writes it again.

use ruff_python_ast::{Expr, Number, Stmt, StmtFunctionDef};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

/// What a parameter is when a host may leave it out: each has a field in the struct of its verb.
#[derive(Clone, Copy, PartialEq)]
enum Fielded {
  Str,
  Int,
  Float,
  Bool,
  /// Any value of the engine, a callable among them: a show of the engine, or a function of the
  /// host.
  Object,
  Text,
  Strs,
  /// A callable that may be none.
  MaybeCallable,
  /// A text that may be None, where None is a word of its own beside the default.
  MaybeStr,
}

impl Fielded {
  fn of(annotation: &str) -> Fielded {
    match annotation {
      // A host gives a word as text; a Text is for the words of the engine alone.
      "str" | "str | Text" => Fielded::Str,
      "str | None" => Fielded::MaybeStr,
      "int" => Fielded::Int,
      "float" | "float | None" => Fielded::Float,
      "bool" => Fielded::Bool,
      "object"
      | "type[T] | object"
      | "Show"
      | "Filter"
      | "Callable[[str], str]"
      | "dict[str, int]"
      | "tuple[int | None, ...] | bool | None" => Fielded::Object,
      "Text" => Fielded::Text,
      "list[str]" => Fielded::Strs,
      "Show | None" | "Filter | None" | "Callable[[str], Ear] | None" => Fielded::MaybeCallable,
      _ => panic!("the build holds no kind of parameter for `{annotation}`"),
    }
  }

  /// The type a host of rust gives the parameter in, when it has no default.
  fn given(self) -> &'static str {
    match self {
      Fielded::Str => "&str",
      Fielded::Int => "i64",
      Fielded::Float => "f64",
      Fielded::Bool => "bool",
      Fielded::Object => "Object",
      Fielded::MaybeCallable => "Option<Object>",
      Fielded::MaybeStr => "Option<&str>",
      Fielded::Text => "&Text",
      Fielded::Strs => "&[&str]",
    }
  }

  /// The type of the field that holds the parameter.
  fn field(self) -> &'static str {
    match self {
      Fielded::Str => "Option<String>",
      Fielded::Int => "Option<i64>",
      Fielded::Float => "Option<f64>",
      Fielded::Bool => "Option<bool>",
      Fielded::Object | Fielded::MaybeCallable => "Option<Object>",
      Fielded::MaybeStr => "Option<Option<String>>",
      Fielded::Text => "Option<Text>",
      Fielded::Strs => "Option<Vec<String>>",
    }
  }

  /// The value that goes in, from what a host of rust gave.
  fn word(self, name: &str) -> String {
    match self {
      Fielded::Str => format!("Object::string({name})"),
      Fielded::Int => format!("Object::int({name})"),
      Fielded::Float => format!("Object::float({name})"),
      Fielded::Bool => format!("Object::bool({name})"),
      Fielded::Object => name.to_owned(),
      Fielded::MaybeCallable => format!("{name}.unwrap_or_else(Object::none)"),
      Fielded::MaybeStr => format!("{name}.map_or_else(Object::none, Object::string)"),
      Fielded::Text => format!("{name}.object()"),
      Fielded::Strs => format!("Object::list({name}.iter().map(|one| Object::string(*one)))"),
    }
  }

  /// The value that goes in, from a field that holds something.
  fn held(self, name: &str) -> String {
    match self {
      Fielded::MaybeCallable => name.to_owned(),
      Fielded::Strs => format!("Object::list({name}.into_iter().map(Object::string))"),
      _ => self.word(name),
    }
  }

  /// The type JavaScript gives the parameter in.
  fn script(self) -> &'static str {
    match self {
      Fielded::Str => "string",
      Fielded::Int | Fielded::Float => "number",
      Fielded::Bool => "boolean",
      Fielded::Object | Fielded::MaybeCallable => "unknown",
      Fielded::MaybeStr => "string | null",
      Fielded::Text => "TextValue",
      Fielded::Strs => "string[]",
    }
  }

  /// How the door to TypeScript carries the value in.
  fn carried(self) -> &'static str {
    match self {
      Fielded::MaybeStr => "Word::Maybe",
      Fielded::Text => "Word::Text",
      _ => "Word::Plain",
    }
  }
}

/// What a parameter is. An ear and a template have no field, so a type holds that no default of
/// either exists.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
  Fielded(Fielded),
  Ear,
  Template,
}

impl Kind {
  fn of(annotation: &str) -> Kind {
    match annotation {
      "Ear" => Kind::Ear,
      "Template" => Kind::Template,
      _ => Kind::Fielded(Fielded::of(annotation)),
    }
  }

  /// The type a host of rust gives the parameter in, when it has no default.
  fn given(self) -> &'static str {
    match self {
      Kind::Fielded(one) => one.given(),
      Kind::Ear => "Box<dyn Ear>",
      Kind::Template => "&[(&str, Object)]",
    }
  }

  /// The value that goes in, from what a host of rust gave.
  fn word(self, name: &str) -> String {
    match self {
      Kind::Fielded(one) => one.word(name),
      Kind::Ear => format!("handed({name}, false)"),
      Kind::Template => format!("templated({name})"),
    }
  }

  /// The type JavaScript gives the parameter in.
  fn script(self) -> &'static str {
    match self {
      Kind::Fielded(one) => one.script(),
      Kind::Ear => "Generator<unknown, unknown, unknown> | NativeEar",
      Kind::Template => "Array<[string, unknown]>",
    }
  }

  /// How the door to TypeScript carries the value in.
  fn carried(self) -> &'static str {
    match self {
      Kind::Fielded(one) => one.carried(),
      Kind::Ear => "Word::Plain",
      Kind::Template => "Word::Template",
    }
  }
}

/// What an act comes to.
#[derive(Clone, Copy)]
enum Came {
  Object,
  Unit,
  Exit,
  /// A text, or nothing.
  MaybeStr,
}

/// What a verb gives.
#[derive(Clone, Copy)]
enum Answer {
  Unit,
  Str,
  Float,
  Strs,
  Fact,
  /// A fact, or nothing for a name of no act.
  MaybeFact,
  Facts,
  Turns,
  Text,
  Object,
  Act(Came),
}

impl Answer {
  fn of(annotation: &str) -> Answer {
    match annotation {
      "None" => Answer::Unit,
      "str" => Answer::Str,
      "float" => Answer::Float,
      "list[str]" => Answer::Strs,
      "Fact" => Answer::Fact,
      "Question" => Answer::MaybeFact,
      "list[Fact]" => Answer::Facts,
      "list[Turn]" => Answer::Turns,
      "Text" => Answer::Text,
      "object" | "Show" | "Filter" | "dict" | "dict[str, str]" | "Standing" | "list[Entry]" => {
        Answer::Object
      }
      "Act" | "Act[object]" | "Act[T]" | "Act[Never]" | "Act[Standing]" => {
        Answer::Act(Came::Object)
      }
      "Act[None]" => Answer::Act(Came::Unit),
      "Act[Exit]" => Answer::Act(Came::Exit),
      "Act[str | None]" => Answer::Act(Came::MaybeStr),
      _ => panic!("the build holds no kind of answer for `{annotation}`"),
    }
  }

  /// The type a host of rust is given.
  fn rust(self) -> &'static str {
    match self {
      Answer::Unit => "()",
      Answer::Str => "String",
      Answer::Float => "f64",
      Answer::Strs => "Vec<String>",
      Answer::Fact => "Fact",
      Answer::MaybeFact => "Option<Fact>",
      Answer::Facts => "Vec<Fact>",
      Answer::Turns => "Vec<Object>",
      Answer::Text => "Text",
      Answer::Object => "Object",
      Answer::Act(Came::Object) => "Act<'_, Object>",
      Answer::Act(Came::Unit) => "Act<'_, ()>",
      Answer::Act(Came::Exit) => "Act<'_, Exit>",
      Answer::Act(Came::MaybeStr) => "Act<'_, Option<String>>",
    }
  }

  /// The type JavaScript is given.
  fn script(self) -> &'static str {
    match self {
      Answer::Unit => "void",
      Answer::Str => "string",
      Answer::Float => "number",
      Answer::Strs => "string[]",
      Answer::Fact => "[string, string, string, ...unknown[]]",
      Answer::MaybeFact => "[string, string, string, ...unknown[]] | null",
      Answer::Facts => "Array<[string, string, string, ...unknown[]]>",
      Answer::Turns => {
        "Array<['user' | 'assistant', string, [number, number, number, number, number] | null, unknown]>"
      }
      Answer::Text => "TextValue",
      Answer::Object => "unknown",
      Answer::Act(Came::Object) => "Act & PromiseLike<unknown>",
      Answer::Act(Came::Unit) => "Act & PromiseLike<null>",
      Answer::Act(Came::Exit) => "Act & PromiseLike<ExitValue>",
      Answer::Act(Came::MaybeStr) => "Act & PromiseLike<string | null>",
    }
  }
}

/// One parameter of a verb that a host must give.
struct Parameter {
  name: String,
  kind: Kind,
}

/// One parameter of a verb that has a default, which puts it in the struct of the verb.
struct Defaulted {
  name: String,
  kind: Fielded,
  /// The default, as rust makes it, of a parameter with a default that stands before the star
  /// parameter of its verb, which goes by its place so that the words of the star follow it.
  placed: Option<String>,
}

/// One verb of the contract.
pub(crate) struct Verb {
  name: String,
  /// The first sentence of its docstring.
  doc: String,
  /// The parameters that have no default, in their order.
  needed: Vec<Parameter>,
  /// The parameters that have a default, in their order.
  optional: Vec<Defaulted>,
  /// Its star parameter, which takes the words that follow the others.
  rest: Option<Parameter>,
  answer: Answer,
}

impl Verb {
  fn read(source: &str, def: &StmtFunctionDef) -> Verb {
    let text = |expr: &Expr| source[expr.range()].to_owned();
    let annotated = |one: &ruff_python_ast::Parameter| {
      Kind::of(&one.annotation.as_deref().map(text).unwrap_or_default())
    };
    let given = &def.parameters;
    let starred = given.vararg.is_some();
    let positional = given.posonlyargs.len() + given.args.len();
    let (mut needed, mut optional) = (Vec::new(), Vec::new());
    for (at, one) in
      given.posonlyargs.iter().chain(&given.args).chain(&given.kwonlyargs).enumerate()
    {
      let name = one.parameter.name.as_str().to_owned();
      match (one.default.as_deref(), annotated(&one.parameter)) {
        (None, kind) => needed.push(Parameter { name, kind }),
        (Some(value), Kind::Fielded(kind)) => {
          let placed = (starred && at < positional).then(|| made(value));
          optional.push(Defaulted { name, kind, placed });
        }
        (Some(_), Kind::Ear | Kind::Template) => {
          panic!("the build holds no default of an ear or of a template")
        }
      }
    }
    let rest = given
      .vararg
      .as_deref()
      .map(|one| Parameter { name: one.name.as_str().to_owned(), kind: annotated(one) });
    let answer = Answer::of(&def.returns.as_deref().map(text).unwrap_or_else(|| "None".to_owned()));
    Verb { name: def.name.as_str().to_owned(), doc: doc(def), needed, optional, rest, answer }
  }

  /// The name of the struct of its defaults, when it has one.
  fn with(&self) -> Option<String> {
    self
      .optional
      .first()
      .map(|_| self.name.split('_').map(|part| part[..1].to_uppercase() + &part[1..]).collect())
  }
}

/// Every verb of the contract, in the order it gives them.
pub(crate) fn verbs(source: &str) -> Vec<Verb> {
  let module = parse_module(source).expect("the contract parses").into_syntax();
  let mut named: Vec<(&StmtFunctionDef, &StmtFunctionDef)> = Vec::new();
  let before = module.body.iter().take_while(|stmt| !matches!(stmt, Stmt::ClassDef(_)));
  for def in
    before.filter_map(|stmt| if let Stmt::FunctionDef(def) = stmt { Some(def) } else { None })
  {
    match named.iter_mut().find(|(first, _)| first.name.as_str() == def.name.as_str()) {
      Some((_, last)) => *last = def,
      None => named.push((def, def)),
    }
  }
  named
    .into_iter()
    .map(|(first, last)| Verb { doc: doc(first), ..Verb::read(source, last) })
    .collect()
}

/// The first sentence of the docstring of a function.
fn doc(def: &StmtFunctionDef) -> String {
  match def.body.first() {
    Some(Stmt::Expr(said)) => match said.value.as_ref() {
      Expr::StringLiteral(one) => one.value.to_str().lines().next().unwrap_or_default().to_owned(),
      _ => String::new(),
    },
    _ => String::new(),
  }
}

/// Each constant of the contract that is a number or a text, as a constant of rust with the
/// sentence that says it, which the door to TypeScript gives, and the function that gives each to
/// the module of python.
pub(crate) fn constants(source: &str) -> String {
  let module = parse_module(source).expect("the contract parses").into_syntax();
  let mut out = String::new();
  let mut python = String::new();
  for (stmt, said) in module.body.iter().zip(module.body.iter().skip(1)) {
    let (Stmt::AnnAssign(one), Stmt::Expr(said)) = (stmt, said) else { continue };
    let (Expr::Name(name), Some(value), Expr::StringLiteral(doc)) =
      (one.target.as_ref(), one.value.as_deref(), said.value.as_ref())
    else {
      continue;
    };
    let (kind, value) = match value {
      Expr::NumberLiteral(number) => match &number.value {
        Number::Int(int) => ("i64", int.to_string()),
        Number::Float(float) => ("f64", format!("{float:?}")),
        Number::Complex { .. } => continue,
      },
      Expr::StringLiteral(text) => ("&str", format!("{:?}", text.value.to_str())),
      _ => continue,
    };
    out!(
      out,
      "/// {}\n#[cfg_attr(feature = \"typescript\", napi_derive::napi)]\npub const {1}: {kind} = {value};",
      doc.value.to_str(),
      name.id
    );
    out!(python, "  module.add({0:?}, {0})?;", name.id.as_str());
  }
  out!(
    out,
    "/// Each constant of the contract, added to the module of python.\n#[cfg(feature = \"python\")]\npub(crate) fn \
     constants(module: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {{\n  use \
     pyo3::types::PyModuleMethods;\n{python}  Ok(())\n}}"
  );
  out
}

/// The methods of the engine, one per verb.
pub(crate) fn methods(verbs: &[Verb]) -> String {
  let mut out = String::from("impl Engine {\n");
  for verb in verbs {
    let mut given: Vec<String> =
      verb.needed.iter().map(|one| format!("{}: {}", one.name, one.kind.given())).collect();
    let mut words: Vec<String> = verb.needed.iter().map(|one| one.kind.word(&one.name)).collect();
    words.extend(verb.optional.iter().filter_map(|one| {
      let held = one.kind.held("one");
      one
        .placed
        .as_ref()
        .map(|value| format!("match with.{} {{ Some(one) => {held}, None => {value} }}", one.name))
    }));
    if let Some(rest) = &verb.rest {
      let typed = if rest.kind == Kind::Fielded(Fielded::Str) { "&[&str]" } else { "Vec<Object>" };
      given.push(format!("{}: {typed}", rest.name));
    }
    if let Some(with) = verb.with() {
      given.push(format!("with: verbs::{with}"));
    }
    out!(out, "  /// {}", verb.doc);
    out!(
      out,
      "  pub fn {}(&mut self, {}) -> Result<{}, Fault> {{",
      verb.name,
      given.join(", "),
      verb.answer.rust()
    );
    let mutable = if verb.rest.is_some() { "mut " } else { "" };
    out!(out, "    let {mutable}args: Vec<Object> = vec![{}];", words.join(", "));
    match &verb.rest {
      Some(rest) if rest.kind == Kind::Fielded(Fielded::Str) => {
        out!(out, "    args.extend({}.iter().map(|one| Object::string(*one)));", rest.name);
      }
      Some(rest) => {
        out!(out, "    args.extend({});", rest.name);
      }
      None => {}
    }
    let mutable = if verb.with().is_some() { "mut " } else { "" };
    out!(out, "    let {mutable}kwargs: Vec<(&str, Object)> = Vec::new();");
    for one in verb.optional.iter().filter(|one| one.placed.is_none()) {
      let held = one.kind.held("one");
      out!(out, "    if let Some(one) = with.{0} {{ kwargs.push(({0:?}, {held})); }}", one.name);
    }
    out!(out, "    let got = self.verb({:?}, args, kwargs)?;", verb.name);
    let answered =
      if let Answer::Act(_) = verb.answer { "self.awaited(got)" } else { "Plain::plain(got)" };
    out!(out, "    {answered}\n  }}\n");
  }
  out.push_str("}\n");
  out
}

/// A default of the contract as rust makes it: a number, a text, a boolean or nothing.
fn made(value: &Expr) -> String {
  match value {
    Expr::NumberLiteral(number) => match &number.value {
      Number::Int(int) => format!("Object::int({int})"),
      Number::Float(float) => format!("Object::float({float:?})"),
      Number::Complex { .. } => panic!("the build holds no complex default"),
    },
    Expr::StringLiteral(text) => format!("Object::string({:?})", text.value.to_str()),
    Expr::BooleanLiteral(one) => format!("Object::bool({})", one.value),
    Expr::NoneLiteral(_) => "Object::none()".to_owned(),
    _ => panic!(
      "the build holds a default of a number, a text, a boolean or nothing before a star alone"
    ),
  }
}

/// The struct of the defaults of each verb that has one.
pub(crate) fn structs(verbs: &[Verb]) -> String {
  let texts = verbs.iter().flat_map(|verb| &verb.optional).any(|one| one.kind == Fielded::Text);
  let used = if texts { "use crate::value::{Object, Text};" } else { "use crate::value::Object;" };
  let mut out = format!("{used}\n\n");
  for verb in verbs {
    let Some(with) = verb.with() else { continue };
    out!(
      out,
      "/// What [`Engine::{0}`](crate::Engine::{0}) takes beside the words it needs: none leaves the default of the \
       engine.",
      verb.name
    );
    out!(out, "#[derive(Debug, Clone, Default)]\npub struct {with} {{");
    for one in &verb.optional {
      out!(out, "  pub {}: {},", one.name, one.kind.field());
    }
    out!(out, "}}\n");
  }
  out
}

/// The methods of the engine that the door to TypeScript gives, one per verb.
pub(crate) fn scripted(verbs: &[Verb]) -> String {
  let mut out = String::from("#[napi]\nimpl JsEngine {\n");
  for verb in verbs {
    let mut given: Vec<String> =
      verb.needed.iter().map(|one| format!("{}: Unknown<'env>", one.name)).collect();
    let mut typed: Vec<String> =
      verb.needed.iter().map(|one| format!("{}: {}", one.name, one.kind.script())).collect();
    let words: Vec<String> =
      verb.needed.iter().map(|one| format!("({}, {})", one.name, one.kind.carried())).collect();
    let rest = match &verb.rest {
      Some(rest) => {
        given.push(format!("{}: Option<Vec<Unknown<'env>>>", rest.name));
        typed.push(format!("{}?: {}[]", rest.name, rest.kind.script()));
        rest.name.clone()
      }
      None => "None".to_owned(),
    };
    let options = match verb.with() {
      Some(_) => {
        given.push("options: Option<JsObject<'env>>".to_owned());
        let fields: Vec<String> =
          verb.optional.iter().map(|one| format!("{}?: {}", one.name, one.kind.script())).collect();
        typed.push(format!("options?: {{ {} }} | null", fields.join("; ")));
        "options"
      }
      None => "None",
    };
    let keys: Vec<String> = verb
      .optional
      .iter()
      .filter(|one| one.placed.is_none())
      .map(|one| format!("({:?}, {})", one.name, one.kind.carried()))
      .collect();
    // A word with a default that stands before the star goes by its place, with the default of the
    // contract.
    let placed: Vec<String> = verb
      .optional
      .iter()
      .filter_map(|one| {
        let value = one.placed.as_ref()?;
        Some(format!("({:?}, {}, {value})", one.name, one.kind.carried()))
      })
      .collect();
    let (called, answer) = match verb.answer {
      Answer::Act(_) => ("acted", "napi::Result<JsAct>"),
      _ => ("plain", "napi::Result<Value>"),
    };
    out!(out, "  /// {}", verb.doc);
    out!(
      out,
      "  #[napi(ts_args_type = {:?}, ts_return_type = {:?})]",
      typed.join(", "),
      verb.answer.script()
    );
    // A verb that takes no word of JavaScript names no lifetime of its env.
    let env = if given.is_empty() { "" } else { "'env" };
    out!(
      out,
      "  pub fn {}<{env}>(&self, env: &{env} Env, {}) -> {answer} {{",
      verb.name,
      given.join(", ")
    );
    // A field whose value is a word of the same name takes the short form, as clippy asks.
    let field = |name: &str, value: &str| {
      if name == value { name.to_owned() } else { format!("{name}: {value}") }
    };
    let (rest, options) = (field("rest", &rest), field("options", options));
    out!(
      out,
      "    self.{called}(*env, {:?}, Words {{ given: vec![{}], placed: &[{}], {rest}, {options}, \
       keys: &[{}] }})\n  }}\n",
      verb.name,
      words.join(", "),
      placed.join(", "),
      keys.join(", ")
    );
  }
  out.push_str("}\n");
  out
}

/// The methods of the engine that the door to python gives, one per verb: the words it needs by
/// position, then the words of its star parameter, then each word with a default by its name alone,
/// as the options of TypeScript are.
pub(crate) fn pythonic(verbs: &[Verb]) -> String {
  let mut out = String::from("#[pymethods]\nimpl PyEngine {\n");
  for verb in verbs {
    let mut signature: Vec<String> = verb.needed.iter().map(|one| one.name.clone()).collect();
    let mut given: Vec<String> =
      verb.needed.iter().map(|one| format!("{}: Value", one.name)).collect();
    // A word with a default that stands before the star stands before it here too, as in python.
    let (placed, named): (Vec<_>, Vec<_>) =
      verb.optional.iter().partition(|one| one.placed.is_some());
    signature.extend(placed.iter().map(|one| format!("{} = None", one.name)));
    given.extend(placed.iter().map(|one| format!("{}: Option<Value>", one.name)));
    match &verb.rest {
      Some(rest) => {
        signature.push(format!("*{}", rest.name));
        given.push(format!("{}: Vec<Value>", rest.name));
      }
      None if verb.with().is_some() => signature.push("*".to_owned()),
      None => {}
    }
    // A word whose None is a word of its own is left out apart from None, so a None that python
    // gives goes in as None, and a word left out leaves the default of the engine.
    signature.extend(named.iter().map(|one| match one.kind {
      Fielded::MaybeStr => format!("{} = Maybe(None)", one.name),
      _ => format!("{} = None", one.name),
    }));
    given.extend(named.iter().map(|one| match one.kind {
      Fielded::MaybeStr => format!("{}: Maybe", one.name),
      _ => format!("{}: Option<Value>", one.name),
    }));
    let mut words: Vec<String> = verb.needed.iter().map(|one| format!("{}.0", one.name)).collect();
    words.extend(placed.iter().filter_map(|one| {
      one.placed.as_ref().map(|value| format!("{}.map_or_else(|| {value}, |one| one.0)", one.name))
    }));
    let keys: Vec<String> = named
      .iter()
      .map(|one| match one.kind {
        Fielded::MaybeStr => format!("({0:?}, {0}.0)", one.name),
        _ => format!("({0:?}, {0})", one.name),
      })
      .collect();
    let (answer, made) = match verb.answer {
      Answer::Act(_) => ("PyAct", "PyAct::of(slf, got)?"),
      _ => ("Value", "Value(got)"),
    };
    out!(out, "  /// {}", verb.doc);
    out!(out, "  #[pyo3(signature = ({}))]", signature.join(", "));
    out!(
      out,
      "  fn {}(slf: &Bound<'_, Self>, {}) -> PyResult<{answer}> {{",
      verb.name,
      given.join(", ")
    );
    let mutable = if verb.rest.is_some() { "mut " } else { "" };
    out!(out, "    let {mutable}args: Vec<Object> = vec![{}];", words.join(", "));
    if let Some(rest) = &verb.rest {
      out!(out, "    args.extend({}.into_iter().map(|one| one.0));", rest.name);
    }
    out!(out, "    let kwargs: [(&str, Option<Value>); {}] = [{}];", keys.len(), keys.join(", "));
    out!(
      out,
      "    let kwargs = kwargs.into_iter().filter_map(|(key, one)| Some((key, one?.0))).collect();"
    );
    out!(out, "    let got = slf.borrow().said({:?}, args, kwargs)?;", verb.name);
    out!(out, "    Ok({made})\n  }}\n");
  }
  out.push_str("}\n");
  out
}
