//! The one piece of python that the crate writes, and what it gives the host, by name.

use std::convert::Infallible;

use crate::value::Object;

/// The one piece of python the crate writes: it loads the engine, the Kernel, the checkpointer and
/// the rule of the record, each in a namespace of its own, and gives what the host calls in the
/// sandbox by.
///
/// It gives the park too: a generator that stands in front of an ear of the outside and holds back
/// from it each act that is parked when the act is put to it, and the parked names, which the host
/// sets at boot and clears at a retry. When a retried act is not done and no ear took it in this
/// life, the park puts it to its ear at the next fact that the ear hears, in the order of the ears,
/// before that fact. The park waits for what its ear waits for, and for every fact while it holds
/// an act back.
///
/// The bound of the turn of the sandbox holds while the Kernel steps the word of a rung, in
/// `starts` and `carry`, and what held before holds again after: a live word keeps its bound, and
/// so does each word that a replay runs again, while the replay, the seed and the gather of a
/// checkpoint around them are the engine's own work, which the host runs unbounded. The bound is
/// the crate's, so the opening sets it, and the Kernel, which python runs too, holds none.
///
/// A step of a word that frees the coroutine of a call of an async function with its body never
/// started is refused at its end. Python warns of such a coroutine, where monty raises nothing and
/// tells it by `monty.unawaited`, so the opening gives the Kernel a `stepped` that asks it before
/// and after each step: a step that runs inside another leaves what was freed before it to that
/// other step.
///
/// A Kernel that raises loses the frame of each word that it stepped, so before a fresh Kernel
/// takes the next act, each run that is not over ends with a refusal that says its word was
/// lost, each open wants of it first, so the chain holds no rung as running and takes its next
/// step. The rung of each run hears the done of its run, so it ends with that refusal.
///
/// It holds the entries that `seed` of the checkpointer makes from the data of a checkpoint, and
/// gives the host the kind and the name of the fact of each alone: the data of a checkpoint can
/// hold each turn of a long life, so it comes in one time and its entries never go out. Boot is
/// given those entries, which it takes once, before the entries of the record that the host hands
/// in.
///
/// Boot is given those entries and the entries of the record, which it says again and keeps what
/// the life says after them, and feed of the engine is given each entry that a host feeds a life
/// that stands, in the same form.
///
/// It gives boot, in the place of a value of a record that no later life makes again as it was,
/// what stands for it by the rule of `furb.record`, which each build reads the same: a callable,
/// which went out by a number of the process that made it, stands as a value equal to each
/// callable, and a text in the words of an act that a word made, which holds the address of an
/// object, stands as a value equal to each text that differs from it in its addresses alone. The
/// heap of a later life is no function of the record, so the act that a word makes again agrees
/// with the record.
///
/// The module of the engine holds the names python gives a module before the engine runs, so a word
/// reads them in a chain as it reads them in python. No loader made the
/// module here, so it has no spec and no loader. Python binds `__debug__` among its builtins, and
/// the sandbox binds it in its main module alone.
pub(super) const OPENING: &str = r#"from string.templatelib import Interpolation, Template
from monty import unawaited as __unawaited

__engine = {"__debug__": True, "__doc__": None, "__package__": "furb", "__spec__": None, "__loader__": None}
exec(__source, __engine)
__record = {}
exec(__record_source, __record)
__kernel = {}
exec(__kernel_source, __kernel)
__checkpointer = {}
exec(__checkpointer_source, __checkpointer)
__checkpointers = {__checkpointer_source: __checkpointer}

def __checkpointed(name, /, *args, by=__checkpointer_source):
  # A carry is gathered by the checkpointer of the texts that it goes to, which the record names, so each later
  # build carries a record the same, while the checkpointer of the record seeds it.
  if by not in __checkpointers:
    __checkpointers[by] = {}
    exec(by, __checkpointers[by])
  return __checkpointers[by][name](__engine, *args)

def __bounding(step):
  def bounded(*args, **kwargs):
    was = __turn(True, args[1])
    try:
      return step(*args, **kwargs)
    finally:
      __turn(was)
  return bounded

for __step in ("starts", "carry"):
  setattr(__kernel["Running"], __step, __bounding(getattr(__kernel["Running"], __step)))
__stepping = []

def __stepped(running, step, *args):
  # Monty tells the coroutines that it freed with their body never started since the last call, so what it freed
  # before a step is the share of the step that holds this one, or of no step.
  taken = __unawaited()
  if __stepping:
    __stepping[-1] += taken
  __stepping.append([])
  try:
    return step(*args)
  finally:
    __kernel["dangling"](running.names, __stepping.pop() + __unawaited())

setattr(__kernel["Running"], "stepped", __stepped)
__parked = set()

def __parking(ear):
  held, heard, wait = [], None, None
  try:
    while True:
      if heard is not None and heard[1] in __parked and heard is __engine["get"](heard[1]):
        held.append(heard)
      else:
        for a in [x for x in held if x[1] not in __parked]:
          held.remove(a)
          on = __engine["scope"](a[1])
          starts = [x[1] for x in __engine["transcript"](on, "started")].count(a[1])
          again = [x[1] for x in __engine["transcript"](on, "journal") if x[0] == "started"].count(a[1])
          taken = starts > again
          if not __engine["over"](a[1]) and not taken:
            said = ear.send(a)
            while isinstance(said, tuple):
              said = ear.send((yield said))
            wait = said
        said = ear.send(heard)
        while isinstance(said, tuple):
          said = ear.send((yield said))
        wait = said
      heard = yield (None if held else wait)
  except StopIteration:
    return

__fresh = {"kernel": lambda: __kernel["kernel"](__engine)}
__seeded = []

def __seed(data):
  __seeded[:] = __checkpointed("seed", data)
  return [x[:2] for (x,) in __seeded]

def __boot(entries, **ears):
  # The rungs that start the parts that live and the wakes end the seed of a carry and start the work of the life. A
  # life that crosses says them after its fact made, as facts of its own that its record keeps, so each later open
  # replays them and their work from the record, an act of the World that a part makes as it starts among them. A
  # record that crossed before keeps none of them, and its open plays them as the end of its seed.
  woke = len(__seeded)
  while woke and __seeded[woke - 1][0][0] == "wake":
    woke -= 1
  while woke and __seeded[woke - 1][0][0] == "rung" and len(__seeded[woke - 1][0]) > 4:
    woke -= 1
  tail = [x[0] for x in __seeded[woke:]]
  made = int(bool(entries) and entries[0][0][0] == "made")
  # A rung that ends the seed takes the next free name when the life says it, so the record holds it by its words.
  key = lambda a: (a[0], *a[2:]) if a[0] == "rung" else tuple(a)
  kept = bool(tail) and any(key(x[0]) == key(tail[0]) for x in entries[made:])
  late = made == len(entries) and bool(tail)
  start = [] if kept or late else __seeded[woke:]
  record = __record["recorded"](__engine, __seeded[:woke] + entries[:made] + start + entries[made:])
  __seeded.clear()
  root = __engine["boot"](record, **ears, birth=__checkpointer["birth"](__engine))
  for kind, about, by, *words in tail if late else ():
    with __engine["site"].set(by):
      if kind == "wake":
        __engine["say"](kind, about, *words)
      else:
        __engine["rung"](*words[1:], on=words[0])
  return root

def __settles(got):
  # The Kernel says why an await of the operator cannot end.
  if (no := __kernel["unsettled"](__engine, "", got)) is not None:
    raise no

def __lasting(name, ear):
  told = False
  while True:
    heard, wait = None, None
    try:
      said = ear.send(None)
      while True:
        if not isinstance(said, tuple):
          wait = said
        heard = yield said
        said = ear.send(heard)
    except StopIteration:
      return
    except Exception as fault:
      why = f"The ear {name} raised {type(fault).__name__}: {fault}"
      get, over = __engine["get"], __engine["over"]
      if isinstance(heard, tuple) and isinstance(wait, (set, frozenset)) and heard[0] in wait:
        if get(heard[1]) is heard and not over(heard[1]):
          yield "done", heard[1], __engine["Refused"](why)
      ear = __fresh[name]() if name in __fresh else ear if __again(name) else None
      if not told:
        told = True
        then = "It starts again and takes the next act." if ear is not None else "It cannot start again."
        try:
          __engine["prompt"](None, f"{why}. {then}", to=__engine["OPERATOR"], on=__engine["ROOT"])
        except Exception:
          pass
      if ear is None:
        return

{
  "names": lambda: {name: x for name, x in __engine.items() if not name.startswith("_") and callable(x)},
  "call": lambda name, /, *args, **kwargs: __engine[name](*args, **kwargs),
  "checkpoint": __checkpointed,
  "kernel": __fresh["kernel"](),
  "site": lambda: __engine["site"].get(),
  "speaks": lambda by: __engine["site"].set(by),
  "spoke": lambda token: __engine["site"].reset(token),
  "word": lambda word, names: eval(word, __engine, names),
  "template": lambda held: Template(*[x if isinstance(x, str) else Interpolation(*x) for x in held]),
  "shown": lambda x: None if callable(x) else repr(x),
  "base": lambda cls: cls.__bases__[0],
  "parking": __parking,
  "park": lambda names: __parked.update(names),
  "unpark": lambda name: __parked.discard(name),
  "lasting": __lasting,
  "unmade": lambda: __record["Recorded"](),
  "seed": __seed,
  "boot": __boot,
  "fed": lambda entries: __engine["feed"](__record["recorded"](__engine, entries)),
  "held": lambda: __engine["flying"](),
  "settles": __settles,
}"#;

/// What the opening gave, by name: the callables of the sandbox that the host calls it by, each
/// held under its handle.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct Opened {
  /// The names of the engine that are callable, read again after boot, which binds some of them.
  pub(super) names: Object,
  /// A name of the engine, called where it stands when it is called.
  pub(super) call: Object,
  /// A function of the checkpointer, `seed` or `gathered`, called by its name on the names of the
  /// engine.
  pub(super) checkpoint: Object,
  /// The Kernel of the crate, which boot is given as an ear.
  pub(super) kernel: Object,
  /// Who speaks: `site`, read.
  pub(super) site: Object,
  /// Who speaks from now on: `site`, set, which gives the token that resets it.
  pub(super) speaks: Object,
  /// Who spoke before: `site`, reset by a token.
  pub(super) spoke: Object,
  /// A word run in the names of the engine, with values bound.
  pub(super) word: Object,
  /// A template string of the sandbox, made from the parts of a template string of the host.
  pub(super) template: Object,
  /// Nothing for a callable, and the representation of anything else.
  pub(super) shown: Object,
  /// The class a class was made with.
  pub(super) base: Object,
  /// An ear wrapped so that it holds each act of the park until the park lets it go.
  pub(super) parking: Object,
  /// Names put into the park.
  pub(super) park: Object,
  /// A name taken out of the parked names of the park.
  pub(super) unpark: Object,
  /// An ear wrapped so that it starts again after it raises.
  pub(super) lasting: Object,
  /// What stands for a callable of the record of an earlier life: a value equal to each callable.
  pub(super) unmade: Object,
  /// The entries that `seed` of the checkpointer makes from the data of a checkpoint, kept in the
  /// sandbox for boot, and the kind and the name of the fact of each, which it gives.
  pub(super) seed: Object,
  /// Boot of the engine on the entries that the seed kept and then on the entries given, with
  /// each text in the words of an act that a word made, that holds the address of an object, in
  /// the place of a value equal to each text that differs from it in its addresses alone.
  pub(super) boot: Object,
  /// The entries that follow the tip of the life, given to feed of the engine as boot is given a
  /// record, which says them again at once.
  pub(super) fed: Object,
  /// The names of the acts that the life holds from the outside, each that an ear of the outside
  /// took and that is not done.
  pub(super) held: Object,
  /// What raises the refusal of an await of the operator that cannot end, as the Kernel says it.
  pub(super) settles: Object,
}

impl Default for Opened {
  /// Nothing under each name, as a life of nothing holds it.
  fn default() -> Self {
    let Ok(none) = Self::of(|_| Ok::<_, Infallible>(Object::none()));
    none
  }
}

impl Opened {
  /// The callables of the opening, each by its name.
  pub(super) fn of<E>(mut at: impl FnMut(&'static str) -> Result<Object, E>) -> Result<Self, E> {
    Ok(Self {
      names: at("names")?,
      call: at("call")?,
      checkpoint: at("checkpoint")?,
      kernel: at("kernel")?,
      site: at("site")?,
      speaks: at("speaks")?,
      spoke: at("spoke")?,
      word: at("word")?,
      template: at("template")?,
      shown: at("shown")?,
      base: at("base")?,
      parking: at("parking")?,
      park: at("park")?,
      unpark: at("unpark")?,
      lasting: at("lasting")?,
      unmade: at("unmade")?,
      seed: at("seed")?,
      boot: at("boot")?,
      fed: at("fed")?,
      held: at("held")?,
      settles: at("settles")?,
    })
  }
}
