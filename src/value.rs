//! What crosses between the engine and a host, which is what monty carries, and the engine's own classes as a
//! host holds them.
//!
//! Monty carries a value of the sandbox out as itself: a number, a text, a list, a tuple, a map, an instance of
//! a class with its fields, an exception with what it was made with. That is [`Object`], monty's own, and the
//! crate adds no value model beside it. What it adds is the reading of three classes the engine defines, since a
//! host wants a `Text` as a text and not as an instance named `Text`: [`Text`], [`Exit`], and [`Fault`], which is
//! any exception by its name and what it was made with.
//!
//! The way in is narrower than the way out: monty makes no instance of a class of the sandbox on a host's behalf.
//! So an instance of a class of the engine goes in as its name and its fields, in a map marked `is`, and the
//! stand-in in the sandbox makes the instance. That is the one rule of the crossing that is not monty's own.

use monty_types::unstable::{self, MontyGraph, MontyNode, NodeId};
pub use monty_types::{MontyObject as Object, ObjectRef};

/// The key of the one mark: a map whose `is` names a class of the engine is an instance of it, by its fields.
pub const IS: &str = "is";

/// A text of the engine: the content at a path.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Text {
  pub path: String,
  pub content: String,
}

impl Text {
  pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
    Text { path: path.into(), content: content.into() }
  }

  /// The text an object is, when it is an instance of `Text`.
  pub fn of(said: ObjectRef<'_>) -> Option<Text> {
    if said.type_name() != "Text" {
      return None;
    }
    Some(Text {
      path: field(&said, "path")?.as_str()?.to_owned(),
      content: field(&said, "content")?.as_str()?.to_owned(),
    })
  }

  /// The text as it goes in: its name and its fields.
  pub fn object(&self) -> Object {
    marked(
      "Text",
      [
        ("path", Object::string(self.path.clone())),
        ("content", Object::string(self.content.clone())),
      ],
    )
  }
}

/// What a command came to: its code, which is nothing for one ended at its timeout, and its two streams.
#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
  pub code: Option<i64>,
  pub stdout: Text,
  pub stderr: Text,
}

impl Exit {
  /// The exit an object is, when it is an instance of `Exit`.
  pub fn of(said: ObjectRef<'_>) -> Option<Exit> {
    if said.type_name() != "Exit" {
      return None;
    }
    Some(Exit {
      code: field(&said, "code").and_then(|one| one.as_int()),
      stdout: Text::of(field(&said, "stdout")?)?,
      stderr: Text::of(field(&said, "stderr")?)?,
    })
  }

  /// The exit as it goes in: its name and its fields.
  pub fn object(&self) -> Object {
    marked(
      "Exit",
      [
        ("code", self.code.map_or_else(Object::none, Object::int)),
        ("stdout", self.stdout.object()),
        ("stderr", self.stderr.object()),
      ],
    )
  }
}

/// An exception, by its name and what it was made with, which is how one is read across the boundary and made
/// again on the other side.
#[derive(Debug, Clone)]
pub struct Fault {
  pub name: String,
  pub args: Vec<Object>,
}

impl Fault {
  pub fn new(name: impl Into<String>, args: Vec<Object>) -> Self {
    Fault { name: name.into(), args }
  }

  /// A refusal of the engine, with why.
  pub fn refused(why: impl Into<String>) -> Self {
    Fault::new("Refused", vec![Object::string(why.into())])
  }

  /// The fault an object is: an instance of an exception class of the sandbox, which carries its `args`, or a
  /// builtin exception, which monty carries by its type and its one message.
  pub fn of(said: ObjectRef<'_>) -> Option<Fault> {
    if said.type_name() == "Exception" {
      let text = said.py_repr();
      let (name, arg) = text.split_once('(').unwrap_or((&text, ""));
      // The interpreter shows some of its exceptions under their module, `asyncio.exceptions.CancelledError`,
      // and a name is bare on both sides of the boundary.
      let name = name.rsplit('.').next().unwrap_or(name);
      let arg = arg.trim_end_matches(')').trim_matches('\'');
      let args = if arg.is_empty() { vec![] } else { vec![Object::string(arg)] };
      return Some(Fault::new(name, args));
    }
    if matches!(said.type_name(), "dict" | "list" | "tuple") {
      return None;
    }
    let args = field(&said, "args")?.items()?.into_iter().map(|one| one.to_owned()).collect();
    Some(Fault::new(said.type_name(), args))
  }

  /// The fault as it goes in: its name and what it was made with.
  pub fn object(&self) -> Object {
    marked(&self.name, [("args", Object::list(self.args.iter().cloned()))])
  }

  /// What it says: its arguments, as text.
  pub fn message(&self) -> String {
    self
      .args
      .iter()
      .map(|one| one.as_ref().as_str().map_or_else(|| one.py_repr(), str::to_owned))
      .collect::<Vec<_>>()
      .join(" ")
  }
}

impl std::fmt::Display for Fault {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}: {}", self.name, self.message())
  }
}

impl std::error::Error for Fault {}

/// One field of an instance or a map, by its name.
pub fn field<'a>(said: &ObjectRef<'a>, name: &str) -> Option<ObjectRef<'a>> {
  said.pairs()?.into_iter().find(|(key, _)| key.as_str() == Some(name)).map(|(_, one)| one)
}

/// One entry of a list or a tuple, by its place.
pub fn entry<'a>(said: &ObjectRef<'a>, at: usize) -> Option<ObjectRef<'a>> {
  said.items()?.into_iter().nth(at)
}

/// A value as it goes in: every instance of a class of a sandbox in it, wherever it came from, as its name and
/// its fields, since an instance that left a sandbox names a class by an id that no other sandbox holds, and
/// monty makes no instance on a host's behalf. An object of the host, which monty made for the host, and plain
/// data go in as they are. This is the one rule, applied to everything the crate hands in, so a record a World
/// kept of one life opens the next.
pub(crate) fn inward(said: &Object) -> Object {
  let (graph, root) = unstable::into_graph_parts(said.clone());
  walk(&graph, root)
}

fn walk(graph: &MontyGraph, id: NodeId) -> Object {
  match graph.node(id) {
    MontyNode::List(held) => Object::list(held.iter().map(|one| walk(graph, *one))),
    MontyNode::Tuple(held) => Object::tuple(held.iter().map(|one| walk(graph, *one))),
    MontyNode::Dict(held) => {
      Object::dict(held.iter().map(|(key, one)| (walk(graph, *key), walk(graph, *one))))
    }
    MontyNode::ClassInstance { class_type, attrs, .. } => match graph.node(*class_type) {
      MontyNode::ClassType(class) if !class.host_defined => {
        let mut held = vec![(Object::string(IS), Object::string(class.name.clone()))];
        held.extend(attrs.iter().map(|(key, one)| (walk(graph, *key), walk(graph, *one))));
        Object::dict(held)
      }
      _ => graph.value(id).to_owned(),
    },
    _ => graph.value(id).to_owned(),
  }
}

/// An instance of a class of the engine, as it goes in: a map marked with the name of its class.
pub fn marked<'a>(name: &str, fields: impl IntoIterator<Item = (&'a str, Object)>) -> Object {
  let mut held = vec![(Object::string(IS), Object::string(name))];
  held.extend(fields.into_iter().map(|(key, one)| (Object::string(key), one)));
  Object::dict(held)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn an_instance_of_the_engine_goes_in_as_its_name_and_its_fields() {
    let sent = Text::new("a.txt", "one\n").object();
    let sent = sent.as_ref();
    assert_eq!(field(&sent, IS).and_then(|one| one.as_str()), Some("Text"));
    assert_eq!(field(&sent, "content").and_then(|one| one.as_str()), Some("one\n"));
    let fault = Fault::refused("no").object();
    assert_eq!(field(&fault.as_ref(), IS).and_then(|one| one.as_str()), Some("Refused"));
  }

  #[test]
  fn a_builtin_exception_is_read_by_its_type_and_its_message() {
    let held = Object::exception(monty_types::ExcType::ValueError, Some("bad".to_owned()));
    let fault = Fault::of(held.as_ref()).unwrap();
    assert_eq!(fault.name, "ValueError");
    assert_eq!(fault.message(), "bad");
  }
}
