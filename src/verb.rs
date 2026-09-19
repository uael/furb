//! The verbs of the engine, as a host that is not python calls them.
//!
//! The operator of a life calls a verb, and the engine runs in a sandbox, so a call is a word that the sandbox
//! runs. This module is the typed way to make such a word: one shape for each verb of the contract, with the
//! words of that verb as its fields, and one reader for what the verb gives. [`crate::Life::calls`] is where the
//! two meet.
//!
//! Every shape takes its defaults from the engine, so a field left unsaid is a word the call never carries, and
//! the engine takes its own. That is what `Prompt { shape: "int", ..Prompt::default() }` means: the shape is
//! said, and the message, the actor and the chain are the engine's to fill.
//!
//! A show and a filter never cross: the engine keeps the one it was given, and no record holds one. So [`Show`]
//! and [`Filter`] are no values of a host. They are what the operator writes in the word, and they stand in the
//! word alone, which is how the engine sees the show the file names and the show a host asks for alike.

use std::fmt;

use crate::{
  fact::{Fact, Value},
  life::Refusal,
  turn::repr,
};

/// The name of an act, which is what a verb gives and what a caller holds of the act.
///
/// It names the act to close, cancel, pause, peek and get, and nothing more: what the act comes to, the life
/// holds under this name, and [`crate::Life::came`] reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Act(pub String);

impl fmt::Display for Act {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl std::ops::Deref for Act {
  type Target = str;

  fn deref(&self) -> &str {
    &self.0
  }
}

impl AsRef<str> for Act {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

/// What a verb is given to say which lines of a text it tells.
///
/// A show is a callable of the engine, so none of these ever crosses a boundary: each stands in the word of the
/// call as the expression that makes it.
#[derive(Debug, Clone, PartialEq)]
pub enum Show {
  /// The first 2000 lines, which is what a read tells without a show of its own.
  Head,
  /// The last 250 lines, which is what a command tells without a show of its own.
  Tail,
  /// Nothing at all, so the act tells neither its open nor its close.
  Hidden,
  /// The lines from one number to another, where a number under zero counts back from the end.
  Span(i64, i64),
  /// The lines a pattern matches.
  Grep(String),
  /// The lines that differ from the lines it is given.
  Differs(Vec<String>),
}

impl Show {
  /// The show as the word that makes it.
  pub fn word(&self) -> String {
    match self {
      Show::Head => "HEAD".to_owned(),
      Show::Tail => "TAIL".to_owned(),
      Show::Hidden => "HIDDEN".to_owned(),
      Show::Span(lo, hi) => format!("span({lo}, {hi})"),
      Show::Grep(pattern) => format!("grep({})", quoted(pattern)),
      Show::Differs(old) => {
        let each: Vec<String> = old.iter().map(|one| quoted(one)).collect();
        format!("differs([{}])", each.join(", "))
      }
    }
  }
}

/// What a chain is given to say which acts of the transcript its turns keep.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Filter {
  /// The acts the filter names.
  pub ids: Vec<String>,
  /// Whether the turns keep the acts it names, or everything but them.
  pub inside: bool,
}

impl Filter {
  /// A filter that keeps the acts it names and everything under them.
  pub fn take(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
    Filter { ids: ids.into_iter().map(Into::into).collect(), inside: true }
  }

  /// A filter that keeps everything but the acts it names.
  #[must_use]
  pub fn outside(mut self) -> Self {
    self.inside = false;
    self
  }

  /// The filter as the word that makes it.
  pub fn word(&self) -> String {
    let mut each: Vec<String> = self.ids.iter().map(|one| quoted(one)).collect();
    if !self.inside {
      each.push("inside=False".to_owned());
    }
    format!("take({})", each.join(", "))
  }
}

/// A text: its path, and what stands at it.
///
/// The engine edits a text into another text, and those edits are the sandbox's own. A host that is not python
/// holds the path and the content, edits the content the way its own language does, and writes the whole of it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Text {
  /// Where the text stands.
  pub path: String,
  /// What stands there.
  pub content: String,
}

impl Text {
  /// A text of a path and what stands at it.
  pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
    Text { path: path.into(), content: content.into() }
  }

  /// The lines of the text, which derive from its content.
  pub fn lines(&self) -> Vec<&str> {
    let mut held = Vec::new();
    let mut start = 0;
    let mut each = self.content.char_indices().peekable();
    while let Some((at, one)) = each.next() {
      if !breaks(one) {
        continue;
      }
      held.push(&self.content[start..at]);
      start = at + one.len_utf8();
      if one == '\r' && each.peek().map(|(_, next)| *next) == Some('\n') {
        each.next();
        start += 1;
      }
    }
    if start < self.content.len() {
      held.push(&self.content[start..]);
    }
    held
  }

  /// The text a value holds, and nothing for a value that is no text.
  pub fn of(got: &Value) -> Option<Self> {
    let (path, content) = (got.field("path")?.as_str()?, got.field("content")?.as_str()?);
    Some(Text::new(path, content))
  }

  /// The text as the word that makes it again.
  pub fn word(&self) -> String {
    format!("Text({}, {})", quoted(&self.path), quoted(&self.content))
  }
}

/// What a command came to: its code, and each of its streams as a text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Exit {
  /// The code of the command, and nothing at all after a timeout.
  pub code: Option<i64>,
  /// What the command wrote to its stdout.
  pub stdout: Text,
  /// What the command wrote to its stderr, which is empty when the command was merged.
  pub stderr: Text,
}

impl Exit {
  /// The exit a value holds, and nothing for a value that is no exit.
  pub fn of(got: &Value) -> Option<Self> {
    let code = match got.field("code")? {
      Value::Int(held) => Some(*held),
      _ => None,
    };
    Some(Exit { code, stdout: Text::of(got.field("stdout")?)?, stderr: Text::of(got.field("stderr")?)? })
  }
}

/// One verb of the engine, as a host calls it.
pub trait Verb {
  /// What the verb gives.
  type Gave;

  /// The word that calls the verb.
  fn word(&self) -> String;

  /// What the verb gave, read off what the word gave.
  fn gave(got: &Value) -> Result<Self::Gave, Refusal>;
}

/// The text at a path, by the lines a show tells.
#[derive(Debug, Clone, Default)]
pub struct Read<'a> {
  /// The path to read, against where the paths of the chain resolve.
  pub path: &'a str,
  /// Which lines of it to tell, and the first 2000 when it says none.
  pub show: Option<Show>,
  /// The chain the read is on.
  pub on: &'a str,
}

impl Verb for Read<'_> {
  type Gave = Text;

  fn word(&self) -> String {
    Call::to("read")
      .word(quoted(self.path))
      .key("show", self.show.as_ref().map(Show::word))
      .key("on", named(self.on))
      .said()
  }

  fn gave(got: &Value) -> Result<Text, Refusal> {
    wanted(got, "text", Text::of(got))
  }
}

/// The content of a text onto the file at its path.
#[derive(Debug, Clone, Default)]
pub struct Write<'a> {
  /// The text to write, which is its path and what goes there.
  pub text: Text,
  /// The chain the write is on.
  pub on: &'a str,
}

impl Verb for Write<'_> {
  type Gave = Text;

  fn word(&self) -> String {
    Call::to("write").word(self.text.word()).key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<Text, Refusal> {
    wanted(got, "text", Text::of(got))
  }
}

/// What an act came to, or what a door answers.
#[derive(Debug, Clone, Default)]
pub struct Peek<'a> {
  /// The act or the door to look at.
  pub at: &'a str,
  /// The chain the peek is on.
  pub on: &'a str,
}

impl Verb for Peek<'_> {
  type Gave = Value;

  fn word(&self) -> String {
    Call::to("peek").word(quoted(self.at)).key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<Value, Refusal> {
    Ok(got.clone())
  }
}

/// The turns of a chain, as the model of that chain reads them.
///
/// A turn crosses plain, since what it holds is the tags of the chain and what a provider answered, and neither
/// is a shape of the engine.
#[derive(Debug, Clone, Default)]
pub struct Turns<'a> {
  /// The chain whose turns these are.
  pub on: &'a str,
}

impl Verb for Turns<'_> {
  type Gave = Vec<Value>;

  fn word(&self) -> String {
    Call::to("turns").key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<Vec<Value>, Refusal> {
    wanted(got, "list of turns", got.as_entries().map(<[Value]>::to_vec))
  }
}

/// The act again, from its name, whole as the life holds it.
#[derive(Debug, Clone, Default)]
pub struct Get<'a> {
  /// The name of the act.
  pub about: &'a str,
}

impl Verb for Get<'_> {
  type Gave = Fact;

  fn word(&self) -> String {
    Call::to("get").word(quoted(self.about)).said()
  }

  fn gave(got: &Value) -> Result<Fact, Refusal> {
    wanted(got, "act", got.as_entries().map(|held| Fact(held.to_vec())))
  }
}

/// The reading of the clock that the World keeps.
#[derive(Debug, Clone, Default)]
pub struct Clock<'a> {
  /// The chain the reading is on.
  pub on: &'a str,
}

impl Verb for Clock<'_> {
  type Gave = f64;

  fn word(&self) -> String {
    Call::to("clock").key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<f64, Refusal> {
    wanted(got, "number", number(got))
  }
}

/// A number the World draws, at least zero and under one.
#[derive(Debug, Clone, Default)]
pub struct Chance<'a> {
  /// The chain the draw is on.
  pub on: &'a str,
}

impl Verb for Chance<'_> {
  type Gave = f64;

  fn word(&self) -> String {
    Call::to("chance").key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<f64, Refusal> {
    wanted(got, "number", number(got))
  }
}

/// What the gate finds against a word, which is nothing at all for a word that may run.
#[derive(Debug, Clone, Default)]
pub struct Gate<'a> {
  /// The word to read.
  pub word: &'a str,
  /// The name of the shape the word must give, and none for a word that answers nothing.
  pub returns: &'a str,
  /// The chain the word would run on.
  pub on: &'a str,
}

impl Verb for Gate<'_> {
  type Gave = Vec<String>;

  fn word(&self) -> String {
    Call::to("gate").word(quoted(self.word)).key("returns", named(self.returns)).key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<Vec<String>, Refusal> {
    let held =
      got.as_entries().map(|each| each.iter().map(|one| one.as_str().unwrap_or_default().to_owned()).collect());
    wanted(got, "list of findings", held)
  }
}

/// Where the paths of a chain resolve from now on.
#[derive(Debug, Clone, Default)]
pub struct Cd<'a> {
  /// The path the chain stands in from now on.
  pub path: &'a str,
  /// The chain that stands there.
  pub on: &'a str,
}

impl Verb for Cd<'_> {
  type Gave = String;

  fn word(&self) -> String {
    Call::to("cd").word(quoted(self.path)).key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<String, Refusal> {
    wanted(got, "path", got.as_str().map(str::to_owned))
  }
}

/// Where the paths of a chain resolve.
#[derive(Debug, Clone, Default)]
pub struct Cwd<'a> {
  /// The chain that is asked.
  pub on: &'a str,
}

impl Verb for Cwd<'_> {
  type Gave = String;

  fn word(&self) -> String {
    Call::to("cwd").key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<String, Refusal> {
    wanted(got, "path", got.as_str().map(str::to_owned))
  }
}

/// One act, paused, which stops it until a wake.
#[derive(Debug, Clone, Default)]
pub struct Pause<'a> {
  /// The act to pause.
  pub id: &'a str,
}

impl Verb for Pause<'_> {
  type Gave = ();

  fn word(&self) -> String {
    Call::to("pause").word(quoted(self.id)).said()
  }

  fn gave(_got: &Value) -> Result<(), Refusal> {
    Ok(())
  }
}

/// One act, woken, which starts it again where it stopped.
#[derive(Debug, Clone, Default)]
pub struct Wake<'a> {
  /// The act to wake.
  pub id: &'a str,
}

impl Verb for Wake<'_> {
  type Gave = ();

  fn word(&self) -> String {
    Call::to("wake").word(quoted(self.id)).said()
  }

  fn gave(_got: &Value) -> Result<(), Refusal> {
    Ok(())
  }
}

/// One act, cancelled, which ends it and everything it made.
#[derive(Debug, Clone, Default)]
pub struct Cancel<'a> {
  /// The act to cancel.
  pub id: &'a str,
}

impl Verb for Cancel<'_> {
  type Gave = ();

  fn word(&self) -> String {
    Call::to("cancel").word(quoted(self.id)).said()
  }

  fn gave(_got: &Value) -> Result<(), Refusal> {
    Ok(())
  }
}

/// One act, ended with a value, which is how the operator answers a prompt.
#[derive(Debug, Clone, Default)]
pub struct Close<'a> {
  /// What the act is done with.
  pub value: Value,
  /// The act to close.
  pub id: &'a str,
}

impl Verb for Close<'_> {
  type Gave = ();

  fn word(&self) -> String {
    Call::to("close").word(shown(&self.value)).key("id", named(self.id)).said()
  }

  fn gave(_got: &Value) -> Result<(), Refusal> {
    Ok(())
  }
}

/// One tag told to a chain, which is what the model of that chain reads next.
#[derive(Debug, Clone, Default)]
pub struct Tell<'a> {
  /// The name of the tag.
  pub name: &'a str,
  /// The attributes of the tag, each a name and what stands under it.
  pub attrs: Vec<(String, Value)>,
  /// What stands inside the tag, and nothing for a tag that says itself and closes.
  pub body: Option<String>,
}

impl Verb for Tell<'_> {
  type Gave = ();

  fn word(&self) -> String {
    let mut held = Call::to("tell").word(quoted(self.name));
    for (name, one) in &self.attrs {
      held = held.word(format!("({}, {})", quoted(name), shown(one)));
    }
    held.key("body", self.body.as_deref().map(quoted)).said()
  }

  fn gave(_got: &Value) -> Result<(), Refusal> {
    Ok(())
  }
}

/// A wait of so many seconds, which is an act like any other.
#[derive(Debug, Clone, Default)]
pub struct Wait<'a> {
  /// How long the wait is.
  pub seconds: f64,
  /// The chain the wait is on.
  pub on: &'a str,
}

impl Verb for Wait<'_> {
  type Gave = Act;

  fn word(&self) -> String {
    Call::to("wait").word(number_word(self.seconds)).key("on", named(self.on)).said()
  }

  fn gave(got: &Value) -> Result<Act, Refusal> {
    named_act(got)
  }
}

/// One rung of a chain: one word, run in the globals of that chain.
#[derive(Debug, Clone, Default)]
pub struct Rung<'a> {
  /// The word to run, and none to have the actor of the chain write one.
  pub word: &'a str,
  /// The act the rung retells, and none for a rung that retells nothing.
  pub retells: &'a str,
  /// The actor that writes the word, and none for the actor of the chain.
  pub actor: &'a str,
  /// The name of the shape the word must give.
  pub returns: &'a str,
  /// The chain the rung is on.
  pub on: &'a str,
}

impl Verb for Rung<'_> {
  type Gave = Act;

  fn word(&self) -> String {
    Call::to("rung")
      .key("word", named(self.word))
      .key("retells", named(self.retells))
      .key("actor", named(self.actor))
      .key("returns", named(self.returns))
      .key("on", named(self.on))
      .said()
  }

  fn gave(got: &Value) -> Result<Act, Refusal> {
    named_act(got)
  }
}

/// A prompt: the rungs of one actor until the actor closes with the shape the prompt wants.
#[derive(Debug, Clone, Default)]
pub struct Prompt<'a> {
  /// The name of the shape the prompt wants, and none for a prompt that wants nothing.
  pub shape: &'a str,
  /// What the prompt says, and none to have the actor read the transcript alone.
  pub message: &'a str,
  /// The actor the prompt goes to, and none for the actor of the chain.
  pub to: &'a str,
  /// The chain the prompt is on.
  pub on: &'a str,
}

impl Verb for Prompt<'_> {
  type Gave = Act;

  fn word(&self) -> String {
    Call::to("prompt")
      .word(if self.shape.is_empty() { "None".to_owned() } else { quoted(self.shape) })
      .key("message", named(self.message))
      .key("to", named(self.to))
      .key("on", named(self.on))
      .said()
  }

  fn gave(got: &Value) -> Result<Act, Refusal> {
    named_act(got)
  }
}

/// A chain: a transcript of its own, which the acts on it tell and its model reads.
#[derive(Debug, Clone, Default)]
pub struct Chain<'a> {
  /// What the chain is called.
  pub label: &'a str,
  /// The act the chain reads its transcript from, and none for a chain of its own.
  pub source: &'a str,
  /// Which acts of that transcript the chain keeps.
  pub filter: Option<Filter>,
  /// The chain the new chain is made on.
  pub on: &'a str,
}

impl Verb for Chain<'_> {
  type Gave = Act;

  fn word(&self) -> String {
    Call::to("chain")
      .key("label", named(self.label))
      .key("source", named(self.source))
      .key("filter", self.filter.as_ref().map(Filter::word))
      .key("on", named(self.on))
      .said()
  }

  fn gave(got: &Value) -> Result<Act, Refusal> {
    named_act(got)
  }
}

/// A grant: what a chain may spend, and what of it goes to the chains it makes.
#[derive(Debug, Clone, Default)]
pub struct Grant<'a> {
  /// The ceiling in dollars, and nothing for a grant that is no ceiling.
  pub usd: Option<f64>,
  /// The share of the ceiling that a chain made under it is given.
  pub share: Option<f64>,
  /// The chain the grant is on.
  pub on: &'a str,
}

impl Verb for Grant<'_> {
  type Gave = Act;

  fn word(&self) -> String {
    Call::to("grant")
      .key("usd", self.usd.map(number_word))
      .key("share", self.share.map(number_word))
      .key("on", named(self.on))
      .said()
  }

  fn gave(got: &Value) -> Result<Act, Refusal> {
    named_act(got)
  }
}

/// A command: its streams as they come, its exit, and the doors of its streams and of its stdin.
#[derive(Debug, Clone, Default)]
pub struct Bash<'a> {
  /// The command to run.
  pub command: &'a str,
  /// Whether the stdin of the command stays open for what a rung writes to it.
  pub fed: bool,
  /// The seconds the command may take, and none for the timeout the engine names.
  pub timeout: Option<f64>,
  /// Which lines of its stdout it tells, and none for its last 250.
  pub show: Option<Show>,
  /// Which lines of its stderr it tells, and none to have its stderr flow into its stdout.
  pub show_err: Option<Show>,
  /// The chain the command is on.
  pub on: &'a str,
}

impl Verb for Bash<'_> {
  type Gave = Act;

  fn word(&self) -> String {
    Call::to("bash")
      .word(quoted(self.command))
      .key("fed", self.fed.then(|| "True".to_owned()))
      .key("timeout", self.timeout.map(number_word))
      .key("show", self.show.as_ref().map(Show::word))
      .key("show_err", self.show_err.as_ref().map(Show::word))
      .key("on", named(self.on))
      .said()
  }

  fn gave(got: &Value) -> Result<Act, Refusal> {
    named_act(got)
  }
}

/// One call of a verb, as the word that makes it: the plain words first, then the named ones.
struct Call {
  /// The name of the verb.
  name: &'static str,
  /// The words of the call, each as the source that says it.
  words: Vec<String>,
}

impl Call {
  /// One call of the verb of a name.
  fn to(name: &'static str) -> Self {
    Call { name, words: Vec::new() }
  }

  /// One plain word of the call.
  fn word(mut self, one: String) -> Self {
    self.words.push(one);
    self
  }

  /// One named word of the call, and nothing at all when the call says none, so the engine takes its own.
  fn key(mut self, name: &str, one: Option<String>) -> Self {
    if let Some(held) = one {
      self.words.push(format!("{name}={held}"));
    }
    self
  }

  /// The whole call, as the word that makes it.
  fn said(self) -> String {
    format!("{}({})", self.name, self.words.join(", "))
  }
}

/// One value as the word that says it again, which is how a host hands a value to the sandbox.
pub(crate) fn shown(value: &Value) -> String {
  match value {
    Value::None => "None".to_owned(),
    Value::Bool(held) => if *held { "True" } else { "False" }.to_owned(),
    Value::Int(held) => held.to_string(),
    Value::Float(held) => number_word(*held),
    Value::Str(held) => quoted(held),
    Value::List(held) | Value::Tuple(held) => {
      let each: Vec<String> = held.iter().map(shown).collect();
      format!("[{}]", each.join(", "))
    }
    Value::Map(held) => {
      let each: Vec<String> = held.iter().map(|(key, one)| format!("{}: {}", quoted(key), shown(one))).collect();
      format!("{{{}}}", each.join(", "))
    }
    Value::Shape { .. } | Value::Error { .. } | Value::Show | Value::Held(_) => shown(&value.plain()),
  }
}

/// One text as the python literal that says it again.
///
/// The escapes of python and of rust are not the same, so this writes the literal itself: a text of any byte of
/// any character crosses to the sandbox as the text it was.
pub(crate) fn quoted(said: &str) -> String {
  let mut held = String::with_capacity(said.len() + 2);
  held.push('"');
  for one in said.chars() {
    match one {
      '"' => held.push_str("\\\""),
      '\\' => held.push_str("\\\\"),
      '\n' => held.push_str("\\n"),
      '\r' => held.push_str("\\r"),
      '\t' => held.push_str("\\t"),
      one if (one as u32) < 0x20 || one as u32 == 0x7F => held.push_str(&format!("\\x{:02x}", one as u32)),
      one => held.push(one),
    }
  }
  held.push('"');
  held
}

/// One number as the word that says it again, where a whole number is still a number and no integer.
fn number_word(held: f64) -> String {
  if held.is_finite() && held.fract() == 0.0 && held.abs() < 1e15 { format!("{held:.1}") } else { format!("{held:?}") }
}

/// One text as a named word, and nothing at all when the text is empty, so the engine takes its own.
fn named(one: &str) -> Option<String> {
  (!one.is_empty()).then(|| quoted(one))
}

/// The number a value holds, which the engine says as a number of either kind.
fn number(got: &Value) -> Option<f64> {
  match got {
    Value::Float(held) => Some(*held),
    Value::Int(held) => Some(*held as f64),
    _ => None,
  }
}

/// The act a value names, which every verb that makes an act gives.
fn named_act(got: &Value) -> Result<Act, Refusal> {
  wanted(got, "act", got.as_str().map(|held| Act(held.to_owned())))
}

/// What a verb gave, or the refusal that says what stood there instead.
fn wanted<T>(got: &Value, what: &str, held: Option<T>) -> Result<T, Refusal> {
  held.ok_or_else(|| Refusal::Read(format!("the engine gave {}, which is no {what}", repr(got))))
}

/// Whether one character ends a line, which is every line break python reads.
fn breaks(one: char) -> bool {
  matches!(one, '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{1C}' | '\u{1D}' | '\u{1E}' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_field_left_unsaid_is_a_word_the_call_never_carries() {
    assert_eq!(Prompt { shape: "int", ..Prompt::default() }.word(), "prompt(\"int\")");
    assert_eq!(
      Prompt { shape: "int", message: "count the lines", on: "chain://operator.1", ..Prompt::default() }.word(),
      "prompt(\"int\", message=\"count the lines\", on=\"chain://operator.1\")"
    );
    assert_eq!(Prompt::default().word(), "prompt(None)");
  }

  #[test]
  fn a_show_and_a_filter_stand_in_the_word_alone() {
    assert_eq!(
      Read { path: "a.txt", show: Some(Show::Span(1, 10)), ..Read::default() }.word(),
      "read(\"a.txt\", show=span(1, 10))"
    );
    assert_eq!(Read { path: "a.txt", show: Some(Show::Tail), ..Read::default() }.word(), "read(\"a.txt\", show=TAIL)");
    assert_eq!(
      Chain { label: "work", filter: Some(Filter::take(["bash://operator.1.1"]).outside()), ..Chain::default() }.word(),
      "chain(label=\"work\", filter=take(\"bash://operator.1.1\", inside=False))"
    );
  }

  #[test]
  fn a_command_says_its_plain_words_first_and_the_shows_of_its_streams_after() {
    let held = Bash {
      command: "ls -la",
      fed: true,
      timeout: Some(30.0),
      show_err: Some(Show::Hidden),
      on: "chain://operator.1",
      ..Bash::default()
    };
    assert_eq!(held.word(), "bash(\"ls -la\", fed=True, timeout=30.0, show_err=HIDDEN, on=\"chain://operator.1\")");
    assert_eq!(Bash { command: "ls", ..Bash::default() }.word(), "bash(\"ls\")");
  }

  #[test]
  fn a_text_of_any_character_crosses_to_the_sandbox_as_the_text_it_was() {
    assert_eq!(quoted("one\ntwo"), "\"one\\ntwo\"");
    assert_eq!(quoted("a \"word\" and a \\"), "\"a \\\"word\\\" and a \\\\\"");
    assert_eq!(quoted("a bell \u{7}"), "\"a bell \\x07\"");
    assert_eq!(quoted("a face \u{1F600}"), "\"a face \u{1F600}\"");
  }

  #[test]
  fn a_text_holds_its_path_and_what_stands_at_it_and_its_lines_derive_from_that() {
    let held = Text::new("/w/a.txt", "one\ntwo\r\nthree\rfour");
    assert_eq!(held.lines(), ["one", "two", "three", "four"]);
    assert_eq!(Text::new("/w/a.txt", "one\n").lines(), ["one"]);
    assert_eq!(Text::new("/w/a.txt", "").lines(), Vec::<&str>::new());
    assert_eq!(held.word(), "Text(\"/w/a.txt\", \"one\\ntwo\\r\\nthree\\rfour\")");
  }

  #[test]
  fn what_a_verb_gave_is_read_off_the_value_the_word_gave() {
    let text = Value::text("/w/a.txt", "one\n");
    assert_eq!(<Read as Verb>::gave(&text).unwrap(), Text::new("/w/a.txt", "one\n"));
    assert_eq!(<Prompt as Verb>::gave(&Value::Str("prompt://operator.2".to_owned())).unwrap().0, "prompt://operator.2");
    assert_eq!(<Clock as Verb>::gave(&Value::Int(3)).unwrap(), 3.0);
    assert!(<Clock as Verb>::gave(&Value::None).is_err());
  }

  #[test]
  fn an_exit_holds_the_code_of_a_command_and_each_of_its_streams() {
    let held = Value::Shape {
      name: "Exit".to_owned(),
      fields: vec![
        ("code".to_owned(), Value::Int(0)),
        ("stdout".to_owned(), Value::text("bash://operator.1.1/stdout", "hi\n")),
        ("stderr".to_owned(), Value::text("bash://operator.1.1/stderr", "")),
      ],
    };
    let got = Exit::of(&held).unwrap();
    assert_eq!(got.code, Some(0));
    assert_eq!(got.stdout.content, "hi\n");
    assert_eq!(Exit::of(&Value::None), None);
  }

  #[test]
  fn a_value_a_host_hands_over_is_the_word_that_says_it_again() {
    assert_eq!(shown(&Value::None), "None");
    assert_eq!(shown(&Value::Bool(true)), "True");
    assert_eq!(shown(&Value::Float(30.0)), "30.0");
    assert_eq!(shown(&Value::List(vec![Value::Int(1), Value::Str("two".to_owned())])), "[1, \"two\"]");
    assert_eq!(shown(&Value::refused("no file")), "{\"is\": \"Refused\", \"args\": [\"no file\"]}");
  }

  #[test]
  fn a_control_of_the_operator_names_the_act_it_is_over() {
    assert_eq!(Cancel { id: "chain://operator.1" }.word(), "cancel(\"chain://operator.1\")");
    assert_eq!(
      Close { value: Value::Int(3), id: "prompt://operator.2" }.word(),
      "close(3, id=\"prompt://operator.2\")"
    );
    assert_eq!(Close { value: Value::None, id: "" }.word(), "close(None)");
  }

  #[test]
  fn a_tell_says_the_name_of_its_tag_then_its_attributes_then_its_body() {
    let held = Tell {
      name: "noted",
      attrs: vec![("by".to_owned(), Value::Str("operator".to_owned()))],
      body: Some("the work is done".to_owned()),
    };
    assert_eq!(held.word(), "tell(\"noted\", (\"by\", \"operator\"), body=\"the work is done\")");
  }
}
