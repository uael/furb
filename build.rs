//! The build: the contract is read here, and every verb of it becomes a method of the engine and of each door.
//!
//! A verb is a function that `src/furb/engine.pyi` gives before its classes, which is what a model or an operator
//! says; the functions after them are the engine's own. A verb that the contract gives only as overloads is the last
//! of them, the widest, with the docstring of the first.
//!
//! The method of a verb takes the parameters that have no default, in their order, then a list for its star
//! parameter, then one struct of the rest, named after the verb, whose fields are none until a host gives them, so
//! the engine keeps its own defaults. The door to TypeScript takes the same, with an object of options for the struct,
//! and the door to python takes the signature of the contract, where none leaves the default of the engine.
//! What each annotation is, in rust and in TypeScript, is the table of [`Kind::of`] and [`Answer::of`], and an
//! annotation that the tables do not hold stops the build, so no verb of the contract goes without its method.

use std::{env, fmt::Write as _, fs, path::Path};

use ruff_python_ast::{Expr, Stmt, StmtFunctionDef};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

/// What a parameter is.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
  Str,
  Int,
  Float,
  Bool,
  /// Any value of the engine.
  Object,
  Text,
  Strs,
  /// A callable, which a host gives as a value: a show of the engine, or a function of the host.
  Callable,
  /// A callable that may be none.
  MaybeCallable,
  Ear,
  Template,
}

impl Kind {
  fn of(annotation: &str) -> Kind {
    match annotation {
      "str" => Kind::Str,
      "int" => Kind::Int,
      "float" | "float | None" => Kind::Float,
      "bool" => Kind::Bool,
      "object" | "type[T] | object" => Kind::Object,
      "Text" => Kind::Text,
      "list[str]" => Kind::Strs,
      "Show" | "Filter" => Kind::Callable,
      "Show | None" | "Filter | None" | "Callable[[str], Ear] | None" => Kind::MaybeCallable,
      "Ear" => Kind::Ear,
      "Template" => Kind::Template,
      _ => panic!("the build holds no kind of parameter for `{annotation}`"),
    }
  }

  /// The type a host of rust gives the parameter in, when it has no default.
  fn given(self) -> &'static str {
    match self {
      Kind::Str => "&str",
      Kind::Int => "i64",
      Kind::Float => "f64",
      Kind::Bool => "bool",
      Kind::Object | Kind::Callable => "Object",
      Kind::MaybeCallable => "Option<Object>",
      Kind::Text => "&Text",
      Kind::Strs => "&[&str]",
      Kind::Ear => "Box<dyn Ear>",
      Kind::Template => "&[(&str, Object)]",
    }
  }

  /// The type of the field that holds the parameter, when it has a default.
  fn field(self) -> &'static str {
    match self {
      Kind::Str => "Option<String>",
      Kind::Int => "Option<i64>",
      Kind::Float => "Option<f64>",
      Kind::Bool => "Option<bool>",
      Kind::Object | Kind::Callable | Kind::MaybeCallable => "Option<Object>",
      Kind::Text => "Option<Text>",
      Kind::Strs => "Option<Vec<String>>",
      Kind::Ear | Kind::Template => panic!("the build holds no default of an ear or of a template"),
    }
  }

  /// The value that goes in, from what a host of rust gave.
  fn word(self, name: &str) -> String {
    match self {
      Kind::Str => format!("Object::string({name})"),
      Kind::Int => format!("Object::int({name})"),
      Kind::Float => format!("Object::float({name})"),
      Kind::Bool => format!("Object::bool({name})"),
      Kind::Object | Kind::Callable => name.to_owned(),
      Kind::MaybeCallable => format!("{name}.unwrap_or_else(Object::none)"),
      Kind::Text => format!("{name}.object()"),
      Kind::Strs => format!("Object::list({name}.iter().map(|one| Object::string(*one)))"),
      Kind::Ear => format!("self.hosted().ear({name}, false)"),
      Kind::Template => format!("templated({name})"),
    }
  }

  /// The value that goes in, from a field that holds something.
  fn held(self, name: &str) -> String {
    match self {
      Kind::MaybeCallable => name.to_owned(),
      Kind::Strs => format!("Object::list({name}.into_iter().map(Object::string))"),
      _ => self.word(name),
    }
  }

  /// The type JavaScript gives the parameter in.
  fn script(self) -> &'static str {
    match self {
      Kind::Str => "string",
      Kind::Int | Kind::Float => "number",
      Kind::Bool => "boolean",
      Kind::Object | Kind::Callable | Kind::MaybeCallable => "unknown",
      Kind::Text => "TextValue",
      Kind::Strs => "string[]",
      Kind::Ear => "Generator<unknown, unknown, unknown> | NativeEar",
      Kind::Template => "Array<[string, unknown]>",
    }
  }

  /// How the door to TypeScript carries the value in.
  fn carried(self) -> &'static str {
    match self {
      Kind::Text => "Word::Text",
      Kind::Template => "Word::Template",
      _ => "Word::Plain",
    }
  }
}

/// What an act comes to.
#[derive(Clone, Copy)]
enum Came {
  Object,
  Unit,
  Exit,
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
      "object" | "Show" | "Filter" | "dict" | "dict[str, str]" | "Standing" => Answer::Object,
      "Act" | "Act[object]" | "Act[T]" | "Act[Never]" => Answer::Act(Came::Object),
      "Act[None]" => Answer::Act(Came::Unit),
      "Act[Exit]" => Answer::Act(Came::Exit),
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
    }
  }
}

/// One parameter of a verb.
struct Parameter {
  name: String,
  kind: Kind,
  /// Whether it has a default, which puts it in the struct of the verb.
  default: bool,
}

/// One verb of the contract.
struct Verb {
  name: String,
  /// The first sentence of its docstring.
  doc: String,
  parameters: Vec<Parameter>,
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
    let parameters = given
      .posonlyargs
      .iter()
      .chain(&given.args)
      .chain(&given.kwonlyargs)
      .map(|one| Parameter {
        name: one.parameter.name.as_str().to_owned(),
        kind: annotated(&one.parameter),
        default: one.default.is_some(),
      })
      .collect();
    let rest = given.vararg.as_deref().map(|one| Parameter {
      name: one.name.as_str().to_owned(),
      kind: annotated(one),
      default: false,
    });
    let answer = Answer::of(&def.returns.as_deref().map(text).unwrap_or_else(|| "None".to_owned()));
    Verb { name: def.name.as_str().to_owned(), doc: doc(def), parameters, rest, answer }
  }

  fn needed(&self) -> impl Iterator<Item = &Parameter> {
    self.parameters.iter().filter(|one| !one.default)
  }

  fn defaults(&self) -> impl Iterator<Item = &Parameter> {
    self.parameters.iter().filter(|one| one.default)
  }

  /// The name of the struct of its defaults, when it has one.
  fn with(&self) -> Option<String> {
    self
      .defaults()
      .next()
      .map(|_| self.name.split('_').map(|part| part[..1].to_uppercase() + &part[1..]).collect())
  }
}

/// Every verb of the contract, in the order it gives them.
fn verbs(source: &str) -> Vec<Verb> {
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

/// The methods of the engine, one per verb.
fn methods(verbs: &[Verb]) -> String {
  let mut out = String::from("impl Engine {\n");
  for verb in verbs {
    let mut given: Vec<String> =
      verb.needed().map(|one| format!("{}: {}", one.name, one.kind.given())).collect();
    let words: Vec<String> = verb.needed().map(|one| one.kind.word(&one.name)).collect();
    if let Some(rest) = &verb.rest {
      let typed = if rest.kind == Kind::Str { "&[&str]" } else { "Vec<Object>" };
      given.push(format!("{}: {typed}", rest.name));
    }
    if let Some(with) = verb.with() {
      given.push(format!("with: verbs::{with}"));
    }
    let _ = writeln!(out, "  /// {}", verb.doc);
    let _ = writeln!(
      out,
      "  pub fn {}(&mut self, {}) -> Result<{}, Fault> {{",
      verb.name,
      given.join(", "),
      verb.answer.rust()
    );
    let mutable = if verb.rest.is_some() { "mut " } else { "" };
    let _ = writeln!(out, "    let {mutable}args: Vec<Object> = vec![{}];", words.join(", "));
    match &verb.rest {
      Some(rest) if rest.kind == Kind::Str => {
        let _ =
          writeln!(out, "    args.extend({}.iter().map(|one| Object::string(*one)));", rest.name);
      }
      Some(rest) => {
        let _ = writeln!(out, "    args.extend({});", rest.name);
      }
      None => {}
    }
    let mutable = if verb.with().is_some() { "mut " } else { "" };
    let _ = writeln!(out, "    let {mutable}kwargs: Vec<(&str, Object)> = Vec::new();");
    for one in verb.defaults() {
      let held = one.kind.held("one");
      let _ = writeln!(
        out,
        "    if let Some(one) = with.{0} {{ kwargs.push(({0:?}, {held})); }}",
        one.name
      );
    }
    let _ = writeln!(out, "    let got = self.verb({:?}, args, kwargs)?;", verb.name);
    let answered =
      if let Answer::Act(_) = verb.answer { "self.awaited(got)" } else { "Plain::plain(got)" };
    let _ = writeln!(out, "    {answered}\n  }}\n");
  }
  out.push_str("}\n");
  out
}

/// The struct of the defaults of each verb that has one.
fn structs(verbs: &[Verb]) -> String {
  let texts = verbs.iter().flat_map(Verb::defaults).any(|one| one.kind == Kind::Text);
  let used = if texts { "use crate::value::{Object, Text};" } else { "use crate::value::Object;" };
  let mut out = format!("{used}\n\n");
  for verb in verbs {
    let Some(with) = verb.with() else { continue };
    let _ = writeln!(
      out,
      "/// What [`Engine::{0}`](crate::Engine::{0}) takes beside the words it needs: none leaves the default of the \
       engine.",
      verb.name
    );
    let _ = writeln!(out, "#[derive(Debug, Clone, Default)]\npub struct {with} {{");
    for one in verb.defaults() {
      let _ = writeln!(out, "  pub {}: {},", one.name, one.kind.field());
    }
    let _ = writeln!(out, "}}\n");
  }
  out
}

/// The methods of the engine that the door to TypeScript gives, one per verb.
fn scripted(verbs: &[Verb]) -> String {
  let mut out = String::from("#[napi]\nimpl JsEngine {\n");
  for verb in verbs {
    let mut given: Vec<String> =
      verb.needed().map(|one| format!("{}: Unknown<'env>", one.name)).collect();
    let mut typed: Vec<String> =
      verb.needed().map(|one| format!("{}: {}", one.name, one.kind.script())).collect();
    let words: Vec<String> =
      verb.needed().map(|one| format!("({}, {})", one.name, one.kind.carried())).collect();
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
          verb.defaults().map(|one| format!("{}?: {}", one.name, one.kind.script())).collect();
        typed.push(format!("options?: {{ {} }} | null", fields.join("; ")));
        "options"
      }
      None => "None",
    };
    let keys: Vec<String> =
      verb.defaults().map(|one| format!("({:?}, {})", one.name, one.kind.carried())).collect();
    let (called, answer) = match verb.answer {
      Answer::Act(_) => ("acted", "napi::Result<JsAct>"),
      _ => ("plain", "napi::Result<Unknown<'env>>"),
    };
    let _ = writeln!(out, "  /// {}", verb.doc);
    let _ = writeln!(
      out,
      "  #[napi(ts_args_type = {:?}, ts_return_type = {:?})]",
      typed.join(", "),
      verb.answer.script()
    );
    let _ = writeln!(
      out,
      "  pub fn {}<'env>(&self, env: &'env Env, {}) -> {answer} {{",
      verb.name,
      given.join(", ")
    );
    let _ = writeln!(
      out,
      "    self.{called}(env, {:?}, vec![{}], {rest}, {options}, &[{}])\n  }}\n",
      verb.name,
      words.join(", "),
      keys.join(", ")
    );
  }
  out.push_str("}\n");
  out
}

/// The methods of the engine that the door to python gives, one per verb.
fn pythonic(verbs: &[Verb]) -> String {
  let mut out = String::from("#[pymethods]\nimpl PyEngine {\n");
  for verb in verbs {
    let mut signature: Vec<String> = verb.needed().map(|one| one.name.clone()).collect();
    let mut given: Vec<String> =
      verb.needed().map(|one| format!(", {}: Bound<'py, PyAny>", one.name)).collect();
    let rest = match &verb.rest {
      Some(rest) => {
        signature.push(format!("*{}", rest.name));
        given.push(format!(", {}: Bound<'py, PyTuple>", rest.name));
        format!("Some({})", rest.name)
      }
      None => "None".to_owned(),
    };
    for one in verb.defaults() {
      signature.push(format!("{}=None", one.name));
      given.push(format!(", {}: Option<Bound<'py, PyAny>>", one.name));
    }
    let needed: Vec<&str> = verb.needed().map(|one| one.name.as_str()).collect();
    let named: Vec<String> =
      verb.defaults().map(|one| format!("({:?}, {})", one.name, one.name)).collect();
    let _ = writeln!(out, "  /// {}", verb.doc);
    // The method takes each word of the contract as python gives it, however many the verb takes.
    if given.len() > 5 {
      let _ = writeln!(out, "  #[allow(clippy::too_many_arguments)]");
    }
    let _ = writeln!(out, "  #[pyo3(signature = ({}))]", signature.join(", "));
    let _ = writeln!(
      out,
      "  fn {}<'py>(&mut self, py: Python<'py>{}) -> PyResult<Bound<'py, PyAny>> {{",
      verb.name,
      given.concat()
    );
    let _ = writeln!(
      out,
      "    self.said(py, {:?}, vec![{}], {rest}, vec![{}])\n  }}\n",
      verb.name,
      needed.join(", "),
      named.join(", ")
    );
  }
  out.push_str("}\n");
  out
}

fn main() {
  #[cfg(feature = "typescript")]
  napi_build::setup();
  println!("cargo::rerun-if-changed=src/furb/engine.pyi");
  let source =
    fs::read_to_string("src/furb/engine.pyi").expect("the contract is beside the engine");
  let verbs = verbs(&source);
  let out = env::var("OUT_DIR").expect("cargo gives the build a directory");
  let written = |name: &str, text: String| {
    fs::write(Path::new(&out).join(name), text).expect("the build writes")
  };
  written("methods.rs", methods(&verbs));
  written("verbs.rs", structs(&verbs));
  written("ts.rs", scripted(&verbs));
  written("py.rs", pythonic(&verbs));
}
