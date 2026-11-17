//! How each value crosses between the sandbox and the host, both ways, and the numbers by which the
//! host holds what has no way back.

use std::{
  collections::{HashMap, HashSet},
  sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
  },
};

use monty_types::{
  ExcType, MontyUuid,
  unstable::{self, MontyGraph, MontyNode, NodeId},
};

use super::{Outside, hosted::At};
use crate::{
  sand::{Sand, object},
  value::{Fault, IS, Object, ObjectRef, carried, entry, inward, items, marked},
};

/// The next number a callable or a class goes out under, in this process.
static NUMBERS: AtomicI64 = AtomicI64::new(1);

/// No later callable of this process goes out under a number that the image of a life holds.
pub(super) fn numbered_past(n: i64) {
  NUMBERS.fetch_max(n.saturating_add(1), Ordering::Relaxed);
}

/// What the host holds of the sandbox, by which each value crosses.
#[derive(Default)]
pub(super) struct Crossing {
  /// The names of the engine that are callable, and the name of each by its handle.
  pub(super) names: HashMap<String, Object>,
  pub(super) named: HashMap<MontyUuid, String>,
  /// Each callable the engine made and each class a word defined that went out, by the number the
  /// host holds it by, and the number of each by its handle, until the host forgets it.
  pub(super) numbered: HashMap<i64, Object>,
  pub(super) numbers: HashMap<MontyUuid, i64>,
  /// The handles the engine holds for itself, which the host never releases.
  pub(super) kept: HashSet<MontyUuid>,
  /// Whether what comes in is the record of an earlier life, whose numbers name callables of the
  /// process that made it, which no callable of this life is.
  pub(super) recorded: bool,
}

impl Outside {
  /// A value of the sandbox that the host holds no longer, let go of by its handle, unless the
  /// engine holds it for itself or the host holds it by a number.
  pub(super) fn drop_handle(&mut self, sand: &mut Sand, handle: &MontyUuid) {
    if !self.crossing.kept.contains(handle) && !self.crossing.numbers.contains_key(handle) {
      sand.release(handle);
    }
  }

  /// A callable the engine made or a class a word defined, by the number the host holds it by.
  pub(super) fn held(&self, n: i64) -> Result<Object, Fault> {
    self.crossing.numbered.get(&n).cloned().ok_or_else(|| Fault::key_error(n))
  }

  /// A callable the engine made or a class a word defined, forgotten by the host, so the sandbox
  /// holds it no more, and neither does the host hold the fact that went out last, which can name
  /// it by its number.
  pub(super) fn forget(&mut self, sand: &mut Sand, n: i64) {
    self.hosted.told = None;
    if let Some(made) = self.crossing.numbered.remove(&n)
      && let Some(handle) = made.as_ref().handle()
    {
      self.crossing.numbers.remove(&handle);
      sand.release(&handle);
    }
  }

  /// The number a callable or a class goes out under, held until the host forgets it. No two lives
  /// of one process give the same number, since a door holds what crossed by its number beyond the
  /// life it crossed from.
  pub(super) fn number(&mut self, handle: MontyUuid, held: &Object) -> i64 {
    if let Some(n) = self.crossing.numbers.get(&handle) {
      return *n;
    }
    let n = NUMBERS.fetch_add(1, Ordering::Relaxed);
    self.crossing.numbers.insert(handle, n);
    self.crossing.numbered.insert(n, held.clone());
    n
  }

  /// A value as it goes out to the host, as a host is given one, so no walk of a host over it is
  /// deeper or longer than the value bounds. A value with no data form that is not callable is let
  /// go of once the whole value went out, since the value may hold it more than once.
  pub(super) fn outward(&mut self, sand: &mut Sand, value: Object) -> Result<Object, Fault> {
    let value = carried(value);
    if stands(&value) {
      return Ok(value);
    }
    let (graph, root) = unstable::into_graph_parts(value);
    let mut shown = HashMap::new();
    let got = self.goes(sand, &graph, root, &mut shown);
    for held in shown.into_keys() {
      self.drop_handle(sand, &held);
    }
    got
  }

  /// One node of a value as it goes out: a callable or a class of the engine as its name, a
  /// callable the engine made by the number the host holds it by, a class a word defined by that
  /// number with the class it was made with as it goes out, an instance of such a class with its
  /// fields under its class, a map that holds the key `is` as its pairs, and the entries of a
  /// container each as they go out. Anything else goes out as the interpreter carries it, but a
  /// value with no data form that is not callable, which goes out as its representation.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a walk of a value holds its arena, its node and the handles that it showed"
  )]
  pub(super) fn goes(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    id: NodeId,
    shown: &mut HashMap<MontyUuid, Object>,
  ) -> Result<Object, Fault> {
    let key = |one: &NodeId| whole(graph, *one);
    Ok(match graph.node(id) {
      MontyNode::Dict(pairs)
        if pairs.iter().any(|(one, _)| graph.value(*one).as_str() == Some(IS)) =>
      {
        let mut held = Vec::new();
        for (one, value) in pairs {
          held.push(Object::tuple([key(one)?, self.goes(sand, graph, *value, shown)?]));
        }
        marked("dict", [("args", Object::list([Object::list(held)]))])
      }
      MontyNode::Dict(pairs) => {
        let mut held = Vec::new();
        for (one, value) in pairs {
          held.push((key(one)?, self.goes(sand, graph, *value, shown)?));
        }
        Object::dict(held)
      }
      MontyNode::List(held) => Object::list(self.going(sand, graph, held, shown)?),
      MontyNode::Tuple(held) => Object::tuple(self.going(sand, graph, held, shown)?),
      MontyNode::Handle { id: handle, .. } => {
        let held = graph.value(id).to_owned();
        if let Some(name) = self.crossing.named.get(handle) {
          return Ok(marked("name", [("name", Object::string(name))]));
        }
        if let Some(n) = self.crossing.numbers.get(handle) {
          return Ok(marked("made", [("id", Object::int(*n))]));
        }
        if let Some(repr) = shown.get(handle) {
          return Ok(repr.clone());
        }
        let opened = Arc::clone(&self.opened);
        let repr = sand.call(self, &opened.shown, vec![held.clone()], vec![])?;
        if let Some(repr) = repr.as_ref().as_str() {
          let repr = Object::repr(repr);
          shown.insert(*handle, repr.clone());
          return Ok(repr);
        }
        marked("made", [("id", Object::int(self.number(*handle, &held)))])
      }
      MontyNode::ClassType(class) if !class.host_defined => {
        self.class(sand, whole(graph, id)?, class.id, &class.name)?
      }
      MontyNode::ClassInstance { class_type, .. } => match graph.node(*class_type) {
        MontyNode::ClassType(class)
          if !class.host_defined && !self.crossing.named.contains_key(&class.id) =>
        {
          let class = self.class(sand, whole(graph, *class_type)?, class.id, &class.name)?;
          marked("instance", [("class", class), ("value", whole(graph, id)?)])
        }
        _ => whole(graph, id)?,
      },
      _ => whole(graph, id)?,
    })
  }

  /// The entries of a container, each as it goes out.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a walk of a value holds its arena, its node and the handles that it showed"
  )]
  pub(super) fn going(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    held: &[NodeId],
    shown: &mut HashMap<MontyUuid, Object>,
  ) -> Result<Vec<Object>, Fault> {
    held.iter().map(|one| self.goes(sand, graph, *one, shown)).collect()
  }

  /// A class of the sandbox as it goes out: a class of the engine as its name, and a class a word
  /// defined as the number the host holds it by, with its name and the class it was made with as it
  /// goes out.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a class is its value, its handle and its name, which its node holds apart"
  )]
  pub(super) fn class(
    &mut self,
    sand: &mut Sand,
    class: Object,
    handle: MontyUuid,
    name: &str,
  ) -> Result<Object, Fault> {
    if let Some(name) = self.crossing.named.get(&handle) {
      return Ok(marked("name", [("name", Object::string(name))]));
    }
    let n = self.number(handle, &class);
    let opened = Arc::clone(&self.opened);
    let base = sand.call(self, &opened.base, vec![class], vec![])?;
    let base = self.outward(sand, base)?;
    Ok(marked("class", [("id", Object::int(n)), ("name", Object::string(name)), ("base", base)]))
  }

  /// A value of the host as the engine holds it, made in the sandbox from what crossed.
  pub(super) fn inward(&mut self, sand: &mut Sand, value: &Object) -> Result<Object, Fault> {
    let value = inward(value);
    let (graph, root) = unstable::graph_parts(&value);
    self.comes(sand, graph, root)
  }

  /// One node of a value as it comes in: a map that names what it is under `is` made again as that,
  /// and the entries of a container each as they come in. Anything else comes in as it is.
  pub(super) fn comes(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    id: NodeId,
  ) -> Result<Object, Fault> {
    Ok(match graph.node(id) {
      MontyNode::Dict(pairs) => {
        let mark = pairs.iter().find(|(one, _)| graph.value(*one).as_str() == Some(IS));
        if let Some(mark) = mark.and_then(|(_, one)| graph.value(*one).as_str().map(str::to_owned))
          && let Some(made) = self.remade(sand, graph, &mark, pairs)?
        {
          return Ok(made);
        }
        let mut held = Vec::new();
        for (key, one) in pairs {
          held.push((graph.value(*key).to_owned(), self.comes(sand, graph, *one)?));
        }
        Object::dict(held)
      }
      MontyNode::List(held) => Object::list(self.coming(sand, graph, held)?),
      MontyNode::Tuple(held) => Object::tuple(self.coming(sand, graph, held)?),
      MontyNode::Function { name, .. } => {
        self.take(name);
        graph.value(id).to_owned()
      }
      _ => graph.value(id).to_owned(),
    })
  }

  /// The entries of a container, each as it comes in.
  pub(super) fn coming(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    held: &[NodeId],
  ) -> Result<Vec<Object>, Fault> {
    held.iter().map(|one| self.comes(sand, graph, *one)).collect()
  }

  /// What a map that names what it is under `is` is made again as, or nothing when it names nothing
  /// the engine makes, so it comes in as the map it is.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a map of a value is its arena, its mark and its pairs"
  )]
  pub(super) fn remade(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    mark: &str,
    pairs: &[(NodeId, NodeId)],
  ) -> Result<Option<Object>, Fault> {
    let at = |name: &str| {
      pairs.iter().find(|(key, _)| graph.value(*key).as_str() == Some(name)).map(|(_, one)| *one)
    };
    let text = |name: &str| at(name).and_then(|one| graph.value(one).as_str().map(str::to_owned));
    let number = |name: &str| at(name).and_then(|one| graph.value(one).as_int());
    match mark {
      // A name of the engine crosses as its name, so a show of the engine is the engine's own here.
      "name"
        if let Some(held) =
          text("name").and_then(|name| self.crossing.names.get(&name).cloned()) =>
      {
        Ok(Some(held))
      }
      // A callable of the record, which no later life holds: what stands in its place, which is
      // equal to each callable, as the one that the word makes again.
      "made" if self.crossing.recorded => {
        let opened = Arc::clone(&self.opened);
        sand.call(self, &opened.unmade, vec![], vec![]).map(Some)
      }
      // A callable the engine made, or a class a word defined, back from the host by the number it
      // went out under.
      "made" if let Some(n) = number("id") => self.held(n).map(Some),
      // An instance of a class a word defined, back from the host by the number of its class and
      // its fields, made here from them as the interpreter makes one, with no `__init__` run.
      "instance" if let (Some(n), Some(fields)) = (number("class"), at("fields")) => {
        let class = self.held(n)?;
        let MontyNode::Dict(fields) = graph.node(fields) else { return Ok(None) };
        let mut held = Vec::new();
        for (key, one) in fields {
          held.push((graph.value(*key).to_owned(), self.comes(sand, graph, *one)?));
        }
        Ok(Some(Object::class_instance(class, self.fresh(), held)))
      }
      "ear"
        if let (Some(name), Some(started)) =
          (text("name"), at("started").and_then(|one| graph.value(one).as_bool())) =>
      {
        self.take(&name);
        let on = self.fresh();
        self.hosted.standing.insert(on, (name, if started { At::Waiting } else { At::Unborn }));
        Ok(Some(object("Ear", on)))
      }
      "Templated" if let Some(MontyNode::List(held)) = at("parts").map(|one| graph.node(one)) => {
        let mut each = Vec::new();
        for one in held {
          let (MontyNode::List(part) | MontyNode::Tuple(part)) = graph.node(*one) else {
            each.push(graph.value(*one).to_owned());
            continue;
          };
          let [value, expression, conversion, spec] = part[..] else { return Ok(None) };
          each.push(Object::tuple([
            self.comes(sand, graph, value)?,
            graph.value(expression).to_owned(),
            graph.value(conversion).to_owned(),
            graph.value(spec).to_owned(),
          ]));
        }
        let opened = Arc::clone(&self.opened);
        sand.call(self, &opened.template, vec![Object::list(each)], vec![]).map(Some)
      }
      _ => self.made_of(sand, graph, mark, pairs),
    }
  }

  /// An instance of a class of the engine or of a builtin class, made from its name, what it was
  /// made with, and its fields. An exception takes what it was made with by position, and by
  /// position alone: the interpreter makes a builtin one from at most one string. A dataclass takes
  /// its fields by name.
  #[expect(
    clippy::too_many_arguments,
    reason = "the side of the host and the sandbox are lent apart, since each call of the sandbox lends the host again, and a map of a value is its arena, its mark and its pairs"
  )]
  pub(super) fn made_of(
    &mut self,
    sand: &mut Sand,
    graph: &MontyGraph,
    mark: &str,
    pairs: &[(NodeId, NodeId)],
  ) -> Result<Option<Object>, Fault> {
    let builtin =
      matches!(mark, "int" | "float" | "dict" | "tuple") || mark.parse::<ExcType>().is_ok();
    if !builtin && !self.crossing.names.contains_key(mark) {
      return Ok(None);
    }
    let mut args = None;
    let mut fields = Vec::new();
    for (key, one) in pairs {
      match graph.value(*key).as_str() {
        Some(IS) => {}
        Some("args") => args = Some(self.comes(sand, graph, *one)?),
        // The interpreter keeps the chain of an exception, as its cause and its context, in
        // attributes of the instance, which no class takes when it makes one.
        Some(key) if key.starts_with("__") && key.ends_with("__") => {}
        Some(key) => fields.push((key.to_owned(), self.comes(sand, graph, *one)?)),
        None => return Ok(None),
      }
    }
    let args: Vec<Object> = args
      .as_ref()
      .and_then(|one| items(one.as_ref()))
      .map(|held| held.map(|one| one.to_owned()).collect())
      .unwrap_or_default();
    if self.crossing.names.contains_key(mark) {
      let named = fields.iter().map(|(key, one)| (key.as_str(), one.clone())).collect();
      return self.named_call(sand, mark, args, named).map(Some);
    }
    builtin_of(mark, args.first().map(Object::as_ref))
  }
}

/// Whether a part of a value goes out in another form than it stands in, as `goes` says: a map
/// that names what it is, a handle, or a class of a word or an instance of one.
/// Whether each part of a value goes out as it stands, so the value goes out whole, with no copy of
/// each part: a fact of the engine is one, and goes out at each ear that hears it.
pub(super) fn stands(value: &Object) -> bool {
  unstable::graph_parts(value).0.nodes().iter().all(|node| !changes(value, node))
}

fn changes(value: &Object, node: &MontyNode) -> bool {
  let graph = unstable::graph_parts(value).0;
  match node {
    MontyNode::Dict(pairs) => pairs.iter().any(|(one, _)| graph.value(*one).as_str() == Some(IS)),
    MontyNode::Handle { .. } => true,
    MontyNode::ClassType(class) => !class.host_defined,
    MontyNode::ClassInstance { class_type, .. } => {
      matches!(graph.node(*class_type), MontyNode::ClassType(class) if !class.host_defined)
    }
    _ => false,
  }
}

/// A part of a value of the sandbox as it stands, in an arena of its own, with each part that it
/// holds more than once held once. The sandbox gives a part that many parts share, as the class of
/// each instance of it, one time in the arena of a value, and the copy that the interpreter makes
/// of a part walks that arena down to the lowest part that it holds. So the copy of each instance
/// of a long fact would walk the fact, and this copy walks the part alone.
fn whole(graph: &MontyGraph, id: NodeId) -> Result<Object, Fault> {
  let mut arena = MontyGraph::new();
  let root = copied(graph, id, &mut arena, &mut HashMap::new());
  unstable::object_from_graph(arena, root)
    .map_err(|no| Fault::refused(format!("a part does not copy: {no}")))
}

/// One part of a value copied into an arena after the parts that it holds, and where it stands
/// there. `done` holds where each part that holds parts stands, so a part that is held again is
/// not copied again.
fn copied(
  graph: &MontyGraph,
  id: NodeId,
  arena: &mut MontyGraph,
  done: &mut HashMap<NodeId, NodeId>,
) -> NodeId {
  if let Some(at) = done.get(&id) {
    return *at;
  }
  let (mut node, mut holds) = (graph.node(id).clone(), false);
  node.for_each_child_mut(|part| {
    holds = true;
    *part = copied(graph, *part, arena, done);
  });
  let at = arena.push(node);
  if holds {
    done.insert(id, at);
  }
  at
}

/// A builtin value made from its name and the first thing it was made with: a number from its text
/// or its value, a map from its pairs, a tuple from its items, and an exception from its message.
#[expect(
  clippy::map_err_ignore,
  reason = "a text that is no number raises ValueError with that text, as python does"
)]
#[expect(clippy::cast_precision_loss, reason = "python makes a float of an int with the same loss")]
fn builtin_of(mark: &str, first: Option<ObjectRef<'_>>) -> Result<Option<Object>, Fault> {
  Ok(match mark {
    "int" => match first.and_then(|one| one.as_str().map(str::to_owned)) {
      Some(text) => Some(match text.trim().parse::<i64>() {
        Ok(n) => Object::int(n),
        Err(_) => Object::bigint(text.trim().parse().map_err(|_| Fault::value_error(text))?),
      }),
      None => first.map(|one| one.to_owned()),
    },
    "float" => match first.and_then(|one| one.as_str().map(str::to_owned)) {
      Some(text) => Some(Object::float(text.trim().parse().map_err(|_| Fault::value_error(text))?)),
      None => first
        .and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)))
        .map(Object::float),
    },
    "dict" => {
      let pairs = first.and_then(items).into_iter().flatten();
      let pairs = pairs.map(|pair| entry(&pair, 0).zip(entry(&pair, 1)));
      let pairs = pairs.map(|pair| pair.map(|(key, one)| (key.to_owned(), one.to_owned())));
      let pairs = pairs.collect::<Option<Vec<_>>>();
      Some(Object::dict(pairs.ok_or_else(|| Fault::refused("a map comes in as its pairs of two"))?))
    }
    "tuple" => {
      let held = first.and_then(items).into_iter().flatten();
      Some(Object::tuple(held.map(|one| one.to_owned())))
    }
    _ => mark.parse::<ExcType>().ok().map(|kind| {
      Object::exception(
        kind,
        first.map(|one| one.as_str().map_or_else(|| one.py_repr(), str::to_owned)),
      )
    }),
  })
}
