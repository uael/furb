"""A World of this machine for the binding tests, and the two helpers that drive a life on it.

The tests drive one life of the real engine through the module the crate makes, so what they hold it to is the
whole of the binding: a World of python behind the boundary, a Voice the host says into, a gate that reads the
word of a rung, and every value that crosses between them.
"""

from pathlib import Path

from furb_sand import Ask, Fact, Fault, Life, Shape, Voice, line

ROSTER = (("operator", (), 200_000), ("m", ("low",), 200_000))
"""ROSTER is the actors a life of the tests offers: the operator, and one model on one effort."""


class Yard:
  """A World of this machine, for the tests: a directory, and a model that answers by a script.

  It is no part of the crate. What a life may touch is the host's to decide, so the crate says what a World is
  and this is one, written small enough that a reader of the tests can hold the whole of it.
  """

  def __init__(self, at: Path, voice: Voice, words: tuple[str, ...] = ()) -> None:
    self.at = at
    self.voice = voice
    self.words = list(words)
    self.read: list[str] = []
    self.kept: list[object] = []
    self.acts: dict[str, Fact] = {}
    self.asking: tuple[str, Fact] | None = None

  def hears(self, fact: Fact) -> object:
    """One fact, heard, and what the World says of it or the question it must ask first."""
    if fact.question and fact.kind in ("bash", "wait", "prompt"):
      self.acts[fact.about] = fact
    if fact.kind == "stand":
      return Fact("done", fact.about, "", (ROSTER, str(self.at), "m/low"))
    if fact.kind in ("clock", "chance"):
      return Fact("done", fact.about, "", 0.5)
    if fact.kind in ("read", "write"):
      # Where a path resolves is the chain's to say, so it is asked before the disk is touched.
      self.asking = ("path", fact)
      return Ask("cwd", fact.on)
    if fact.kind == "start":
      # A prompt of the operator is answered by whoever holds this World, so the yard answers it at once. A
      # command and a wait are the two an act of this World would start, and neither is written here.
      held = self.acts.get(fact.about)
      if held is not None and held.kind == "prompt":
        self.voice.close(held.about, "the operator says so")
      return None
    if fact.kind == "ask":
      self.read.append(text(fact.words[2]))
      word = self.words.pop(0) if self.words else "close(None)"
      self.voice.fact(Fact("answer", fact.about, "", ("assistant", [word], None, [])))
      return None
    if fact.kind == "keep":
      self.kept.append(line(fact.words[0]))
    return None

  def answered(self, got: object) -> object:
    """The answer to the question the World last asked, and what it says now that it holds it."""
    asking = self.asking
    self.asking = None
    if asking is None:
      return None
    _, held = asking
    at = Path(self.at, str(got or ""))
    if held.kind == "read":
      one = at / str(held.words[1])
      try:
        said: object = Shape("Text", {"path": str(one), "content": one.read_text()})
      except OSError:
        said = Fault("Refused", f"no file at {one}")
      return Fact("done", held.about, "", said)
    text = held.words[1]
    one = at / str(text.path)
    one.parent.mkdir(parents=True, exist_ok=True)
    one.write_text(text.content)
    return Fact("done", held.about, "", Shape("Text", {"path": str(one), "content": text.content}))


def text(got: object) -> str:
  """Every word of a value, run together, which is how a test reads what a model was shown."""
  if isinstance(got, str):
    return got
  if isinstance(got, (list, tuple)):
    return " ".join(text(one) for one in got)
  if isinstance(got, dict):
    return " ".join(text(one) for one in got.values())
  if isinstance(got, Shape):
    return " ".join(text(one) for one in got.fields.values())
  if isinstance(got, Fault):
    return f"{got.name} " + " ".join(text(one) for one in got.args)
  return ""


class Strict:
  """A gate of the tests, which refuses a word that holds BAD, as the harness of the suite does."""

  def gate(self, word: str, ladder: list[str], shape: str) -> list[str]:
    """What the gate finds against a word. Nothing at all means the word may run."""
    return ["BAD in rung"] if "BAD" in word else []


def life(at: Path, words: tuple[str, ...] = (), gate: object = None, record: str | None = None) -> tuple[Life, Yard]:
  """One life of the real engine, on a World of this machine and the words a model answers with."""
  voice = Voice()
  world = Yard(at, voice, words)
  return Life(world, gate=gate, record=record, voice=voice), world


def came(held: Life, act: str, tries: int = 200) -> object:
  """What an act came to, once the host has said everything it owes.

  A host says what it owes first, since a fact it is holding may be the very one that settles the act. Nothing
  waits forever here: a life that never settles is a fault of the test and it says so.
  """
  for _ in range(tries):
    held.heard()
    got = held.came(act)
    if got is not None:
      return got[0]
    held.waits(0.05)
  raise AssertionError(f"{act} never came to anything")
