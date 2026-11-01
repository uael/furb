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
//! engine makes the instance in the sandbox. That is the one rule of the crossing that is not monty's own.

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

  /// The text an object is, when it is an instance of `Text`, or a map that names it, as a text of the engine of
  /// python goes in.
  pub fn of(said: ObjectRef<'_>) -> Option<Text> {
    if said.type_name() != "Text" && field(&said, IS).and_then(|one| one.as_str()) != Some("Text") {
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

/// The most levels of a value that a host is given. Each door walks a value one level at a time on the stack of its
/// thread, so a deeper value can end the process of the host, and monty itself gives a value at most as deep as its
/// recursion lets it. No fact, transcript, turn or standing of the engine comes near this depth.
pub(crate) const DEEPEST: usize = 64;

/// How many times its own parts a value that a host is given may hold at most. Monty gives a part that a value holds
/// more than once as one part, and a host is given each of its places, so a value of few parts can stand for more
/// parts than a host can walk: thirty lists, each of which holds the one before it twice, stand for a billion. A value
/// that holds each of its parts once holds them once, so it is always under this bound.
pub(crate) const REPEATS: usize = 64;

/// What monty gives in the place of a part deeper than its recursion lets it go, which a part deeper than `DEEPEST`
/// is given as too.
pub(crate) const NESTED: &str = "<deeply nested>";

/// What a part is given as in the place of a part past the bound of `REPEATS`.
pub(crate) const REPEATED: &str = "<repeated too often>";

/// A value as a host is given it: whole when it is at most `DEEPEST` levels deep and stands for at most `REPEATS`
/// times its parts, and else cut, each part at the last level a host is given in the place of `NESTED`, and each
/// part past the bound in the place of `REPEATED`, from the first place in the order of a walk. So every walk of a
/// host over the value is bounded by its depth and by its size in the sandbox.
pub(crate) fn carried(value: Object) -> Object {
  let (graph, root) = unstable::graph_parts(&value);
  let (mut height, mut size) = (vec![0usize; graph.len()], vec![0usize; graph.len()]);
  // The arena holds each part after the parts it holds, so one pass gives each part its height and its size.
  for (at, node) in graph.nodes().iter().enumerate() {
    let (mut high, mut many) = (0usize, 1usize);
    node.for_each_child(|one| {
      high = high.max(height[one.index()] + 1);
      many = many.saturating_add(size[one.index()]);
    });
    (height[at], size[at]) = (high, many);
  }
  let room = graph.len().saturating_mul(REPEATS);
  if height[root.index()] < DEEPEST && size[root.index()] <= room {
    return value;
  }
  let (mut cut, mut left) = (MontyGraph::new(), room);
  let root = cutting(graph, root, 0, &mut left, &mut cut, false);
  unstable::object_from_graph(cut, root).unwrap_or_else(|_| Object::repr(NESTED))
}

/// One part of a value as `carried` cuts it into the arena of the cut, at a level, from what is left of its bound.
/// The class of an instance stays a class, since an instance names a class, and only what the class holds is cut.
fn cutting(
  graph: &MontyGraph,
  id: NodeId,
  level: usize,
  left: &mut usize,
  cut: &mut MontyGraph,
  class: bool,
) -> NodeId {
  let node = graph.node(id);
  if !node.is_leaf() && !class {
    if level + 1 >= DEEPEST {
      return cut.push(MontyNode::Repr(NESTED.to_owned()));
    }
    if *left == 0 {
      return cut.push(MontyNode::Repr(REPEATED.to_owned()));
    }
  }
  *left = left.saturating_sub(1);
  let mut held = Vec::new();
  node.for_each_child(|one| held.push(one));
  let instance = matches!(node, MontyNode::ClassInstance { .. });
  let held: Vec<NodeId> = held
    .into_iter()
    .enumerate()
    .map(|(at, one)| cutting(graph, one, level + 1, left, cut, instance && at == 0))
    .collect();
  let mut node = node.clone();
  let mut each = held.into_iter();
  node.for_each_child_mut(|one| {
    if let Some(made) = each.next() {
      *one = made;
    }
  });
  cut.push(node)
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

/// A template string as the engine reads it: its parts in order, as python iterates one, each string as it is and
/// each interpolation as its value, its expression, its conversion and its format spec.
pub(crate) fn templated(parts: impl IntoIterator<Item = Object>) -> Object {
  marked("Templated", [("parts", Object::list(parts))])
}

/// A template string as a host says one that writes each expression beside its value: each pair as the f-string
/// {expression=} makes it, the expression, an equals sign and the repr of the value, with a space between two.
pub(crate) fn paired(pairs: impl IntoIterator<Item = (String, Object)>) -> Object {
  templated(pairs.into_iter().enumerate().flat_map(|(at, (expression, value))| {
    let lead = if at == 0 { "" } else { " " };
    [
      Object::string(format!("{lead}{expression}=")),
      Object::list([value, Object::string(expression), Object::string("r"), Object::string("")]),
    ]
  }))
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A list that holds a list, and so on, as many lists deep as asked, around an empty list.
  fn nested(lists: usize) -> Object {
    let mut graph = MontyGraph::new();
    let mut at = graph.push(MontyNode::List(vec![]));
    for _ in 0..lists {
      at = graph.push(MontyNode::List(vec![at]));
    }
    unstable::object_from_graph(graph, at).unwrap()
  }

  #[test]
  fn a_value_under_both_bounds_is_given_whole() {
    let pairs = [(Object::string("a"), nested(DEEPEST - 3)), (Object::string("b"), Object::int(1))];
    let value = Object::list([Object::dict(pairs), Object::string("c")]);
    assert_eq!(carried(value.clone()), value);
    assert_eq!(carried(nested(DEEPEST - 1)), nested(DEEPEST - 1));
  }

  #[test]
  fn a_value_deeper_than_deepest_is_given_its_first_levels_and_the_mark_of_monty_at_the_last() {
    let mut cut = Object::repr(NESTED);
    for _ in 0..DEEPEST - 1 {
      cut = Object::list([cut]);
    }
    assert_eq!(carried(nested(DEEPEST)), cut);
    assert_eq!(carried(nested(990)), cut);
  }

  #[test]
  fn a_value_that_holds_its_parts_too_often_is_given_within_its_bound() {
    let mut graph = MontyGraph::new();
    let mut at = graph.push(MontyNode::List(vec![]));
    for _ in 0..30 {
      at = graph.push(MontyNode::List(vec![at, at]));
    }
    let room = graph.len() * REPEATS;
    let cut = carried(unstable::object_from_graph(graph, at).unwrap());
    let (graph, _) = unstable::graph_parts(&cut);
    assert!(graph.len() <= 2 * room, "{} parts", graph.len());
    let marks =
      graph.nodes().iter().filter(|one| matches!(one, MontyNode::Repr(t) if t == REPEATED));
    assert!(marks.count() > 0);
  }

  #[test]
  fn a_builtin_exception_is_read_by_its_type_and_its_message() {
    let held = Object::exception(monty_types::ExcType::ValueError, Some("bad".to_owned()));
    let fault = Fault::of(held.as_ref()).unwrap();
    assert_eq!(fault.name, "ValueError");
    assert_eq!(fault.message(), "bad");
  }
}
