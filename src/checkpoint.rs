//! The checkpoint of a life: what each chain that lives holds, as plain data that a build of any version reads, so a
//! later life goes on from it, on another engine too, with no replay of what came before it.
//!
//! The fact is `("checkpoint", "chain1", "store", {"version": v, "engine": e, "entries": [...]})`, where v and e are
//! the version and the engine of the build that wrote it. No contract holds it, so this is the one home of its shape.
//! Its entries are facts that the journal of any engine replays, made by [`SOURCE`] in the names of the
//! engine that writes them, and `("reserve", kind, "store", n)`, which stands for each name of that kind up to n that
//! no entry carries. A later life boots on the entries, each reserve said as a question of the root that the journal
//! skips, so each name of the past stays taken, then on the entries of the record after the fact.

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

/// The kind of an entry of a checkpoint that stands for the names of a kind that no entry carries.
const RESERVE: &str = "reserve";

/// The python that makes the entries, in a copy of the names of the engine, with the definitions of a word as
/// [`definitions`] gives them. Each value that it gives is plain, so a build of any version reads it.
const SOURCE: &str = r##"def _plain(value):
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
  for c in sorted(live, key=_number):
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


def checkpoint(define, floor):
  asked, live, named, values, lost, resumed = _lives()
  defined = {}
  for c in live:
    words = [define(w) for r, w in program(c).items() if (get(r) or ("", "", ""))[2] != "extensions"]
    defined[c] = "\n\n".join(w for w in words if w)
  counts = dict(floor)
  for x in asked:
    counts[x[0]] = max(counts.get(x[0], 0), _number(x[1]))
  entries, reserved = [], {}

  def take(kind, n):
    if reserved.get(kind, 0) < n:
      entries.append(("reserve", kind, "store", n))
      reserved[kind] = n

  for c in live:
    act = get(c)
    take("chain", _number(c) - 1)
    entries.append(act if c == ROOT else ("chain", c, OPERATOR, act[3] if act[3] in live else "", act[4], ""))
    reserved["chain"] = _number(c)
  stands = [x for x in transcript(ROOT, "stand") if question(x) and over(x[1])]
  clocks = [v for x in transcript(ROOT, "clock") if isinstance(v := peek(x[1]), float)]
  for x in stands[-1:]:
    take("stand", _number(x[1]) - 1)
    take("clock", counts.get("clock", 0))
    n = reserved.get("clock", 0) + 1
    entries += [x, ("done", x[1], "store", peek(x[1])), ("clock", f"clock{n}", ROOT, ROOT)]
    entries.append(("done", f"clock{n}", "store", clocks[-1] if clocks else __import__("time").time()))
    reserved["stand"], reserved["clock"] = _number(x[1]), n
  verbs = ("prompt", "bash", "read", "write", "wait", "cd", "chance")
  carried = sorted((one for one in named if get(one)[0] in verbs), key=lambda one: (get(one)[0], _number(one)))
  paused = sorted({scope(one) for one in carried if get(one)[0] == "prompt"}, key=_number)
  entries += [("pause", c, OPERATOR) for c in paused]
  cut = []
  for one in carried:
    act = get(one)
    value = peek(one) if over(one) else None
    if over(one) and not _plain(value):
      continue
    take(act[0], _number(one) - 1)
    entries.append((act[0], one, OPERATOR, *act[3:]))
    reserved[act[0]] = _number(one)
    if over(one):
      entries += [("started", one, "store"), ("done", one, "store", value)]
    elif act[0] == "prompt":
      entries += [x for x in transcript(scope(one)) if x[0] == "started" and x[1] == one and get(x[2]) is None]
    else:
      cut.append(one)
      entries += [("started", one, "store"), ("done", one, "store", Refused(f"{one} was cut by the checkpoint"))]
  for kind, n in counts.items():
    take(kind, n)
  for c in live:
    again = [x for x in resumed if scope(x[1]) == c]
    said = {x[1]: x[3] for x in transcript(c) if x[0] == "ready"}
    word = "\n".join(
      [
        defined.get(c, ""),
        *(f"{name} = {shown}" for name, shown in values[c]),
        "for x in [x for x in transcript(scope(acting())) if x[0] == 'carried'][-1:]:",
        "  say('prefix', x[1], [*transcript(x[1], 'module')[-1:], *x[3], *transcript(x[1], 'stand')[-1:],",
        "    *transcript(x[1], 'pause'), *transcript(x[1], 'run')[-1:]])",
        '  debug(t"{'
        + repr(
          f"A new build goes on from a checkpoint. It lost {lost[c] or 'no name'}, and it cut {[x for x in cut if scope(x) == c] or 'no act'}."
          + (f" It runs again from its top each word that ran: {[x[4] for x in again]}." if again else "")
        )
        + '}")',
        *(f"with site.set({get(x[4])[2]!r}):\n  rung({said.get(x[4], x[5])!r}, on={c!r})" for x in again),
      ]
    ).strip("\n")
    turns = [
      x
      for x in transcript(c)
      if x[0] not in ("module", "carried") and _plain(x) and (not question(x) or x[0] == "reply" or x[1] in carried)
    ]
    held = {ROOT, *live, *carried, *(x[1] for x in turns if question(x))}
    turns = [x for x in turns if x[0] not in ("started", "done") or x[1] in held]
    counts["rung"] = counts.get("rung", 0) + 1
    counts["gate"] = counts.get("gate", 0) + 1
    entries += [
      ("carried", c, "store", turns),
      ("rung", f"rung{counts['rung']}", OPERATOR, c, word, "", ""),
      ("gate", f"gate{counts['gate']}", c, c, word),
      ("done", f"gate{counts['gate']}", "gate", []),
    ]
  return entries
"##;
const WRITE: &str =
  "(exec(__source, __names := {**globals()}), __names['checkpoint'](__define, __floor))[1]";

/// The entry of the checkpoint of a life at its tip, as the store keeps it, on the record that the life stands on.
pub fn written(engine: &mut Engine, record: &[Object]) -> Result<Object, Fault> {
  let define = callable(|args: Vec<Object>| {
    let word = args.first().and_then(|one| one.as_ref().as_str().map(str::to_owned));
    Ok(Object::string(definitions(&word.unwrap_or_default())))
  });
  let words =
    vec![("__source", Object::string(SOURCE)), ("__define", define), ("__floor", taken(record))];
  let entries = engine.word(WRITE, words)?;
  let carried = Object::dict([
    (Object::string("version"), Object::int(VERSION)),
    (Object::string("engine"), Object::string(world::engine())),
    (Object::string("entries"), entries),
  ]);
  let fact = [CHECKPOINT, ROOT, "store"].map(Object::string).into_iter().chain([carried]);
  Ok(Object::list([Object::list(fact)]))
}

/// An entry kept after the last entry of the record at a path, as the store of a life keeps one.
pub fn keep(path: &Path, one: Object) -> Result<(), Fault> {
  match world::database(path) {
    true => Sqlite::open(path)?.keep(one.as_ref()),
    false => Lines::open(path)?.keep(one.as_ref()),
  }
}

/// An entry of a checkpoint as the record reads it: a fact, which the record holds as the mark of a tuple.
fn unmarked(one: ObjectRef<'_>) -> Option<Object> {
  let Some(pairs) = one.pairs() else { return Some(one.to_owned()) };
  let field =
    |name: &str| pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|(_, value)| *value);
  match field("is").and_then(|is| is.as_str()) {
    Some("tuple") => field("args")?.items()?.first().map(|args| args.to_owned()),
    _ => Some(one.to_owned()),
  }
}

/// The highest number that each kind took in a record, its checkpoints among them, as a map. A later life makes again
/// no act that an ear of the outside made, as the merged of a command, so its transcripts do not hold each name that the
/// record took, and the checkpoint keeps each of them taken.
fn taken(record: &[Object]) -> Object {
  let mut most: Vec<(String, i64)> = Vec::new();
  for a in booted(record.to_vec()).iter().filter_map(fact).filter(Fact::question) {
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

/// The entries that the build of the last life on a record made: those after its last checkpoint, or the whole
/// record when it holds none.
pub fn segment(record: &[Object]) -> &[Object] {
  &record[last(record).map_or(0, |at| at + 1)..]
}

/// The record as a life boots on it: the entries of its last checkpoint, each reserve as the questions of the root
/// that it stands for, then the entries after the checkpoint; or the record as it is when it holds none.
pub fn booted(record: Vec<Object>) -> Vec<Object> {
  let Some(at) = last(&record) else { return record };
  let Some(a) = fact(&record[at]) else { return record };
  let held = a.word(0).and_then(|one| one.pairs()).and_then(|pairs| {
    pairs
      .into_iter()
      .find(|(key, _)| key.as_str() == Some("entries"))
      .and_then(|(_, one)| one.items())
  });
  let held: Vec<Object> = held.unwrap_or_default().into_iter().filter_map(unmarked).collect();
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
  out.extend(record.into_iter().skip(at + 1));
  out
}
