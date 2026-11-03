//! The checkpoint of a life: what each chain that lives holds, as plain data that a build of any
//! version of the major reads, so a later life goes on from it, on another engine too, with no
//! replay of what came before it.
//!
//! The fact is `("checkpoint", "chain1", "store", data)`. Its data is a map: `schema`, the version
//! of its own shape, [`SCHEMA`]; `version` and `engine`, the build that wrote it; `taken`, the
//! highest number of each kind; `chains`, each chain that lives with its parent, its label, its
//! source, whether it is paused, its turns, the text of its definitions, its plain values by name,
//! the names that it lost, the acts that the checkpoint cut, and each word that ran on it; and
//! `acts`, each act that a value names, the last stand and the last clock, with its value, or, for
//! an open prompt, with its shape, its message, its actor and the ear that held it. No contract
//! holds it, so this is the one home of its shape. It holds no word that the crate crafts and no
//! word of a verb by its place: `seed` of the engine that opens it makes the entries that a life
//! boots on, so any later engine of the major opens it.
//!
//! A life ends with its checkpoint, which the ear of its store keeps, since a life born from a
//! checkpoint makes acts of its own at its birth; [`keep`] writes one on a record that no life
//! holds. A life of the engine that made each segment of its record boots on no checkpoint: it
//! replays the record whole, as [`crossed`] says, and loses nothing that a checkpoint drops.
//!
//! The checkpoint of the build before the schema holds `entries` in the place of the data: facts
//! that the journal of any engine replays, and `("reserve", kind, "store", n)`, which stands for
//! each name of that kind up to n that no entry carries. A later life boots on them as they stand,
//! so a life that a build of that form handed over goes on.

#![expect(clippy::match_bool, reason = "debt: a match on a bool")]
#![expect(
  clippy::needless_pass_by_value,
  reason = "debt: an argument taken by value and only borrowed"
)]
#![expect(
  clippy::needless_raw_string_hashes,
  reason = "debt: a raw string with hashes that it does not need"
)]
#![expect(
  clippy::redundant_closure_for_method_calls,
  reason = "debt: a closure where a method path serves"
)]

use std::{
  collections::{HashMap, HashSet},
  path::Path,
};

use crate::{
  Engine,
  engine::{ROOT, callable},
  extension::definitions,
  fact::Fact,
  value::{Fault, Object, ObjectRef, entry},
  world::{self, Lines, Sqlite, Store, VERSION},
};

/// The kind of the fact of a checkpoint.
pub const CHECKPOINT: &str = "checkpoint";

/// The version of the shape of the data of a checkpoint, apart from the version of the record.
pub const SCHEMA: i64 = 1;

/// The kind of an entry of a checkpoint of the form before [`SCHEMA`] that stands for the names of
/// a kind that no entry carries.
const RESERVE: &str = "reserve";

/// The python that gathers the data of a checkpoint, in a copy of the names of the engine, with the
/// definitions of a word as [`definitions`] gives them: a view, which makes nothing and says
/// nothing. Each value that it gives is plain, so a build of any version reads it.
const GATHER: &str = r##"def _plain(value):
  if value is None or isinstance(value, (bool, int, float, str, Text, Exit, BaseException)):
    return True
  if isinstance(value, (list, tuple, set)):
    return all(_plain(one) for one in value)
  if isinstance(value, dict):
    return all(_plain(k) and _plain(v) for k, v in value.items())
  return False


def _named(value, found):
  if isinstance(value, str) and get(value) is not None:
    found.add(value)
  elif isinstance(value, (list, tuple, set)):
    for one in value:
      _named(one, found)
  elif isinstance(value, dict):
    for k, v in value.items():
      _named(k, found)
      _named(v, found)
  return found


def _number(name):
  return int(re.sub(r"^[a-z_]+", "", name) or 0)


def _shown(value):
  if isinstance(value, Act):
    return f"Act({str(value)!r})"
  if isinstance(value, list):
    return "[" + ", ".join(_shown(one) for one in value) + "]"
  if isinstance(value, tuple):
    return "(" + ", ".join(_shown(one) for one in value) + ("," if len(value) == 1 else "") + ")"
  if isinstance(value, set):
    return "{" + ", ".join(_shown(one) for one in value) + "}" if value else "set()"
  if isinstance(value, dict):
    return "{" + ", ".join(f"{_shown(k)}: {_shown(v)}" for k, v in value.items()) + "}"
  return repr(value)


def _lives():
  chains = []
  while get(f"chain{len(chains) + 1}") is not None:
    chains.append(f"chain{len(chains) + 1}")
  asked = [get(c) for c in chains] + [x for c in chains for x in transcript(c) if question(x) and x[0] != "chain"]
  opened = [x[1] for x in asked if x[0] == "prompt" and not over(x[1])]
  runs = sorted((x for x in asked if x[0] == "run" and not over(x[1]) and get(x[4])), key=lambda x: _number(x[1]))
  waits = {x[2]: x[4] for x in asked if x[0] == "wants" and not over(x[1])}
  resumed, dropped = [], set()
  for x in runs:
    maker = get(x[4])[2]
    kind = (get(maker) or ("",))[0]
    if maker == OPERATOR or kind == "chain" or kind == "prompt" and not over(maker) and maker not in dropped:
      resumed.append(x)
      if (get(waits.get(x[1], "")) or ("",))[0] == "prompt":
        dropped.add(waits[x[1]])
  opened = [one for one in opened if one not in dropped]
  live = {ROOT} | {scope(one) for one in opened}
  values, lost, named = {}, {}, set(opened)
  while live - values.keys():
    for c in sorted(live - values.keys(), key=_number):
      here = module(c)
      values[c], lost[c] = [], []
      for name, value in here.items():
        if name.startswith("__") or callable(value) or globals().get(name, here) is value:
          continue
        if _plain(value):
          values[c].append((name, _shown(value)))
          _named(value, named)
        else:
          lost[c].append(name)
    live |= {get(one)[1] if get(one)[0] == "chain" else scope(one) for one in named}
  live = sorted(live, key=_number)
  return asked, live, named, values, lost, [x for x in resumed if scope(x[1]) in live]


def gathered(define, floor):
  asked, live, named, values, lost, resumed = _lives()
  taken = dict(floor)
  for x in asked:
    taken[x[0]] = max(taken.get(x[0], 0), _number(x[1]))
  verbs = ("prompt", "bash", "read", "write", "wait", "cd", "chance")
  carried = sorted((one for one in named if get(one)[0] in verbs), key=lambda one: (get(one)[0], _number(one)))
  stands = [x[1] for x in transcript(ROOT, "stand") if question(x) and over(x[1])][-1:]
  clocks = [x[1] for x in transcript(ROOT, "clock") if question(x) and isinstance(peek(x[1]), float)][-1:]
  acts, cut, asking = [], [], {scope(one) for one in carried if get(one)[0] == "prompt"}
  for one in [*stands, *clocks, *carried]:
    act = get(one)
    if act[0] == "prompt" and not over(one):
      held = [x[2] for x in transcript(scope(one)) if x[0] == "started" and x[1] == one and get(x[2]) is None]
      acts.append(
        {"kind": "prompt", "id": one, "on": act[3], "shape": act[4], "message": act[5], "to": act[6], "held": (held or [""])[0]}
      )
    elif over(one):
      if _plain(peek(one)):
        acts.append({"kind": act[0], "id": one, "on": act[3], "value": peek(one)})
    else:
      cut.append(one)
      acts.append({"kind": act[0], "id": one, "on": act[3], "value": Refused(f"{one} was cut by the checkpoint")})
  chains = []
  for c in live:
    act = get(c)
    said = {x[1]: x[3] for x in transcript(c) if x[0] == "ready"}
    words = [define(w) for r, w in program(c).items() if (get(r) or ("", "", ""))[2] != "extensions"]
    turns = [
      x
      for x in transcript(c)
      if x[0] not in ("module", "carried") and _plain(x) and (not question(x) or x[0] == "reply" or x[1] in carried)
    ]
    held = {ROOT, *live, *carried, *(x[1] for x in turns if question(x))}
    chains.append(
      {
        "id": c,
        "on": act[3] if act[3] in live else "",
        "label": act[4],
        "source": act[5],
        "paused": c in asking or paused(c),
        "turns": [x for x in turns if x[0] not in ("started", "done") or x[1] in held],
        "definitions": "\n\n".join(w for w in words if w),
        "values": dict(values[c]),
        "lost": lost[c],
        "cut": [x for x in cut if scope(x) == c],
        "again": [
          {"rung": x[4], "by": get(x[4])[2], "word": said.get(x[4], x[5])} for x in resumed if scope(x[1]) == c
        ],
      }
    )
  return {"taken": taken, "chains": chains, "acts": acts}
"##;
const WRITE: &str =
  "(exec(__source, __names := {**globals()}), __names['gathered'](__define, __floor))[1]";

/// The entry of the checkpoint of a life at its tip, as the store keeps it.
pub fn written(engine: &mut Engine) -> Result<Object, Fault> {
  let define = callable(|args: Vec<Object>| {
    let word = args.first().and_then(|one| one.as_ref().as_str().map(str::to_owned));
    Ok(Object::string(definitions(&word.unwrap_or_default())))
  });
  let floor = engine.taken().clone();
  let words = vec![("__source", Object::string(GATHER)), ("__define", define), ("__floor", floor)];
  let data = engine.word(WRITE, words)?;
  let mut fields = vec![
    (Object::string("schema"), Object::int(SCHEMA)),
    (Object::string("version"), Object::int(VERSION)),
    (Object::string("engine"), Object::string(world::engine())),
  ];
  let pairs = data.as_ref().pairs().unwrap_or_default();
  fields.extend(pairs.into_iter().map(|(key, one)| (key.to_owned(), one.to_owned())));
  let fact =
    [CHECKPOINT, ROOT, "store"].map(Object::string).into_iter().chain([Object::dict(fields)]);
  Ok(Object::list([Object::list(fact)]))
}

impl Engine {
  /// The checkpoint of the life at its tip, as the last entry of its record: the ear of its store
  /// keeps it after each entry that it kept, as it keeps each fact, and the life ends with it, so a
  /// later life on the record opens from it. A life that is born from a checkpoint makes acts of
  /// its own at its birth, so a fact that this life said after the checkpoint would not replay
  /// there. The life ends too when it keeps no record, which it refuses.
  pub fn checkpoint(mut self) -> Result<(), Fault> {
    if !self.hears("store") {
      return Err(Fault::refused("a life that keeps no record keeps no checkpoint"));
    }
    let entry = written(&mut self)?;
    self.word("say('keep', '', __entry)", vec![("__entry", entry)]).map(drop)
  }
}

/// An entry kept after the last entry of the record at a path, as the store of a life keeps one,
/// for a record that no life holds.
pub fn keep(path: &Path, one: Object) -> Result<(), Fault> {
  match world::database(path) {
    true => Sqlite::open(path)?.keep(one.as_ref()),
    false => Lines::open(path)?.keep(one.as_ref()),
  }
}

/// An entry of a checkpoint as the record reads it: a fact, which the record holds as the mark of a
/// tuple.
fn unmarked(one: ObjectRef<'_>) -> Option<Object> {
  let Some(pairs) = one.pairs() else { return Some(one.to_owned()) };
  let field =
    |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|(_, value)| *value);
  match field("is").and_then(|is| is.as_str()) {
    Some("tuple") => field("args")?.items()?.first().map(|args| args.to_owned()),
    _ => Some(one.to_owned()),
  }
}

/// The highest number that each kind took in a record, its checkpoints among them, as a map. A
/// later life makes again no act that an ear of the outside made, as the merged of a command, so
/// its transcripts do not hold each name that the record took, and the checkpoint keeps each of
/// them taken.
pub(crate) fn taken(record: &[Object]) -> Object {
  let opened;
  let record = match last(record) {
    Some(_) => {
      opened = booted(record.to_vec());
      opened.as_slice()
    }
    None => record,
  };
  let mut most: Vec<(String, i64)> = Vec::new();
  for a in record.iter().filter_map(fact).filter(Fact::question) {
    let Some(n) = a.about().strip_prefix(a.kind()).and_then(|one| one.parse::<i64>().ok()) else {
      continue;
    };
    match most.iter_mut().find(|one| one.0 == a.kind()) {
      Some(one) => one.1 = one.1.max(n),
      None => most.push((a.kind().to_owned(), n)),
    }
  }
  Object::dict(most.into_iter().map(|(kind, n)| (Object::string(kind), Object::int(n))))
}

/// The fact of an entry.
fn fact(one: &Object) -> Option<Fact> {
  entry(&one.as_ref(), 0).and_then(Fact::of)
}

/// Where the last checkpoint of a record stands, and nothing when it holds none.
pub fn last(record: &[Object]) -> Option<usize> {
  record.iter().rposition(|one| fact(one).is_some_and(|a| a.kind() == CHECKPOINT))
}

/// The entries that the build of the last life on a record made: those after its last checkpoint,
/// or the whole record when it holds none.
pub fn segment(record: &[Object]) -> &[Object] {
  &record[last(record).map_or(0, |at| at + 1)..]
}

/// Where the checkpoint that a life of this engine boots on stands: the last checkpoint that
/// follows a segment of another engine, as a fact made or the engine of a checkpoint names it.
/// Nothing when this engine made each segment, so the life replays the record whole and loses
/// nothing that a checkpoint drops.
pub fn crossed(record: &[Object]) -> Option<usize> {
  let ours = world::engine();
  let (mut foreign, mut at) = (false, None);
  for (here, a) in record.iter().enumerate().filter_map(|(here, one)| Some((here, fact(one)?))) {
    let word = a.word(0);
    let engine = match a.kind() {
      world::MADE => word.and_then(|one| one.as_str().map(str::to_owned)),
      CHECKPOINT => word.and_then(|one| one.pairs()).and_then(|pairs| {
        let found = pairs.into_iter().find(|(key, _)| key.as_str() == Some("engine"));
        found.and_then(|(_, one)| one.as_str().map(str::to_owned))
      }),
      _ => continue,
    };
    foreign |= engine.is_some_and(|one| one != ours);
    if a.kind() == CHECKPOINT && foreign {
      (foreign, at) = (false, Some(here));
    }
  }
  at
}

/// The record as a life boots on it: the entries of the checkpoint where [`crossed`] says, then the
/// entries after it; or the record as it stands when no segment crosses, so a life of the engine
/// that wrote each checkpoint replays the record whole, each checkpoint a fact that it says again.
pub fn booted(record: Vec<Object>) -> Vec<Object> {
  let Some(at) = crossed(&record) else { return record };
  let mut out = carried(&record[at]);
  out.extend(record.into_iter().skip(at + 1));
  out
}

/// The entries that a checkpoint carries: the fact itself when it holds data, which boot gives
/// `seed` of the engine, or, for the form before [`SCHEMA`], its entries, each reserve as the
/// questions of the root that it stands for.
fn carried(checkpoint: &Object) -> Vec<Object> {
  let Some(a) = fact(checkpoint) else { return Vec::new() };
  let pairs = a.word(0).and_then(|one| one.pairs()).unwrap_or_default();
  let field =
    |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|one| one.1);
  let Some(held) = field("entries").and_then(|one| one.items()) else {
    return vec![checkpoint.clone()];
  };
  let held: Vec<Object> = held.into_iter().filter_map(unmarked).collect();
  let facts: Vec<Fact> = held.iter().filter_map(|one| Fact::of(one.as_ref())).collect();
  let carried: HashSet<String> = facts
    .iter()
    .filter(|a| a.kind() != RESERVE && a.question())
    .map(|a| a.about().to_owned())
    .collect();
  let (mut out, mut reserved) = (Vec::new(), HashMap::<String, i64>::new());
  for a in &facts {
    if a.kind() != RESERVE {
      out.push(Object::list([a.0.clone()]));
      continue;
    }
    let (kind, n) = (a.about(), a.word(0).and_then(|one| one.as_int()).unwrap_or_default());
    let from = reserved.get(kind).copied().unwrap_or_default();
    for name in (from + 1..=n).map(|i| format!("{kind}{i}")).filter(|name| !carried.contains(name))
    {
      out.push(Object::list([Object::list([kind, &name, ROOT, ""].map(Object::string))]));
    }
    reserved.insert(kind.to_owned(), from.max(n));
  }
  out
}
