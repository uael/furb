//! The one piece of python that the crate writes, and what it gives the host, by name.

use std::convert::Infallible;

use crate::value::Object;

/// The one piece of python the crate writes: it loads the engine, the sheet and the Kernel, each in
/// a namespace of its own, and gives what the host calls in the sandbox by.
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
/// A Kernel that raises loses the frame of each word that it stepped, so before a fresh Kernel
/// takes the next act, each run that is not over ends with a refusal that says its word was
/// lost, each open wants of it first, so the chain holds no rung as running and takes its next
/// step. A rung that awaited an act through a wants hears only that act, so it is closed with
/// that refusal, and a rung of any other run hears the done of its run.
///
/// It gives what stands for a value of a record that no later life makes again as it was: a
/// callable, which went out by a number of the process that made it, stands as a value equal to
/// each callable, and a text in the words of an act that a word made, which holds the address of an
/// object, stands as a value equal to each text that differs from it in its addresses alone. The
/// heap of a later life is no function of the record, so the act that a word makes again agrees
/// with the record.
///
/// The module of the engine holds the names python gives a module before the engine runs, so a word
/// reads them in a chain as it reads them in python, and as the gate reads them. No loader made the
/// module here, so it has no spec and no loader. Python binds `__debug__` among its builtins, and
/// the sandbox binds it in its main module alone.
pub(super) const OPENING: &str = r#"from string.templatelib import Interpolation, Template

__engine = {"__debug__": True, "__doc__": None, "__package__": "furb", "__spec__": None, "__loader__": None}
exec(__source, __engine)
__sheet = {}
exec(__sheet_source, __sheet)
__kernel = {}
exec(__kernel_source, __kernel)

def __bounding(step):
  def bounded(*args, **kwargs):
    was = __turn(True)
    try:
      return step(*args, **kwargs)
    finally:
      __turn(was)
  return bounded

for __step in ("starts", "carry"):
  setattr(__kernel["Running"], __step, __bounding(getattr(__kernel["Running"], __step)))
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

class __Recorded:
  def __init__(self, held=None):
    self.held = held

  def __eq__(self, other):
    import re

    if isinstance(other, type(self)):
      return self.held == other.held
    if self.held is None:
      return callable(other)
    unaddressed = lambda text: re.sub(r" at 0x[0-9a-f]+", "", text)
    return isinstance(other, str) and unaddressed(other) == unaddressed(self.held)

  def __repr__(self):
    return "<a callable of an earlier life>" if self.held is None else repr(self.held)

def __recorded(entries):
  import re

  Text = __engine["Text"]

  def again(word):
    if isinstance(word, str) and re.search(r" at 0x[0-9a-f]+", word):
      return __Recorded(word)
    if isinstance(word, Text):
      return Text(word.path, again(word.content))
    if isinstance(word, (tuple, list)):
      return type(word)(map(again, word))
    return word

  return [
    ((*x[:4], *map(again, x[4:])),) if __engine["question"](x) and re.fullmatch(r"[a-z_]+\d+", x[2]) else (x,)
    for (x,) in entries
  ]

__fresh = {"kernel": lambda: __kernel["kernel"](__engine), "gate": lambda: __sheet["gating"](__engine, __gate, __gated)}

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
      if name == "kernel":
        lost = f"{why}, so the word of the rung of this run was lost and runs no more"
        n, refused = 1, __engine["Refused"]
        while get(on := f"chain{n}"):
          n += 1
          for run in [x[1] for x in __engine["transcript"](on, "run") if get(x[1]) is x and not over(x[1])]:
            wants = [x[1] for x in __engine["transcript"](on, "wants") if get(x[1]) is x and x[2] == run]
            wants = [x for x in wants if not over(x)]
            for one in wants:
              yield "done", one, refused(lost)
            yield "done", run, refused(lost)
            if wants and get(rung := get(run)[4]) and not over(rung):
              __engine["close"](refused(lost), rung)
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
  "kernel": __fresh["kernel"](),
  "gate": __fresh["gate"](),
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
  "against": lambda on, word: __sheet["against"](__engine, __gate, on, word) if __gated() else [],
  "lasting": __lasting,
  "unmade": lambda: __Recorded(),
  "recorded": __recorded,
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
  /// The Kernel of the crate, which boot is given as an ear.
  pub(super) kernel: Object,
  /// The gate of the crate, which boot is given as an ear.
  pub(super) gate: Object,
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
  /// What the gate finds against a word on a chain, which makes no fact.
  pub(super) against: Object,
  /// An ear wrapped so that it starts again after it raises.
  pub(super) lasting: Object,
  /// What stands for a callable of the record of an earlier life: a value equal to each callable.
  pub(super) unmade: Object,
  /// The entries of a record, with each text in the words of an act that a word made, that holds
  /// the address of an object, in the place of a value equal to each text that differs from it in
  /// its addresses alone.
  pub(super) recorded: Object,
}

impl Default for Opened {
  /// Nothing under each name, as a life of nothing holds it.
  fn default() -> Opened {
    let Ok(none) = Opened::of(|_| Ok::<_, Infallible>(Object::none()));
    none
  }
}

impl Opened {
  /// The callables of the opening, each by its name.
  pub(super) fn of<E>(mut at: impl FnMut(&'static str) -> Result<Object, E>) -> Result<Opened, E> {
    Ok(Opened {
      names: at("names")?,
      call: at("call")?,
      kernel: at("kernel")?,
      gate: at("gate")?,
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
      against: at("against")?,
      lasting: at("lasting")?,
      unmade: at("unmade")?,
      recorded: at("recorded")?,
    })
  }
}
