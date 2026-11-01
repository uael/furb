"""A life of furb as a program holds it: open it on its record, prompt it, open chains, answer its questions.

`open` opens a life of the engine of the crate on a record, on the ears of the crate and on a console of its own,
which takes each prompt to the operator. A record that holds work goes on: the pending work of an earlier life
resumes at once. A program that runs again on its record makes its prompts, chains and grants in the same order, so
each finds the act of the record that it made before, done or pending, as a word finds its acts again; a call that
does not agree with the record at its place makes a new act, and the act of the record goes on as pending work.
"""

import asyncio
import atexit
import importlib
import re
from collections.abc import AsyncIterator, Callable, Generator
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType

from furb_monty import _monty

engine: ModuleType = importlib.import_module("furb_monty.engine")
"""engine is the engine of the crate, whose names the door gives at its import."""
OPERATOR = "operator"
"""OPERATOR is the actor of a prompt that the operator answers, and the site of every act of the program."""
ENDED = "The life ended."
"""ENDED is what a wait of a program on a life raises once the life ended."""
MADE = ("chain", "prompt", "grant", "rung")
"""MADE are the kinds of the acts that a program makes, which a later run of it finds again in the record."""


def question(kind: str, id: str) -> bool:
  """Whether a name is the name of an act of that kind."""
  return re.fullmatch(kind + r"\d+", id) is not None


def named(shape: object) -> str:
  """The name of a shape, as a prompt carries it."""
  return shape if isinstance(shape, str) else re.sub(r"<class '|'>|[\w.:/]*\.", "", repr(shape))


@dataclass(frozen=True)
class LifeAct:
  """One act of a chain, with named fields: its kind, its name, who made it, its chain, its words, and what it came
  to once it is done."""

  kind: str
  id: str
  by: str
  on: str
  words: tuple
  done: bool
  value: object


class Chain:
  """One chain of a life: a conversation with a model of its own, which a program prompts and puts a ceiling on."""

  def __init__(self, life: Life, id: str) -> None:
    self.life, self.id = life, id

  @property
  def label(self) -> str:
    """The label that the chain was opened with."""
    return str(engine.get(self.id)[4])

  @property
  def parent(self) -> Chain | None:
    """The chain that this chain was opened on, and none for the root."""
    on = engine.get(self.id)[3]
    return Chain(self.life, on) if on else None

  @property
  def actor(self) -> str:
    """The actor that a prompt of this chain goes to when it names none."""
    for one in reversed(engine.transcript(self.id)):
      match one:
        case ("rung", _, "operator", on, word, *_) if on == self.id and str(word).startswith("actor = "):
          return str(word).removeprefix("actor = ").strip("'\"")
    return str(engine.standing()[2])

  @property
  def spent(self) -> float:
    """The dollars of the answers of the models on this chain."""
    return sum(
      float(one[3][2][4])
      for one in engine.transcript(self.id)
      if one[0] == "done" and question("reply", one[1]) and isinstance(one[3], tuple | list) and one[3][2]
    )

  def prompt(self, shape: type | object, message: str, *, to: str | None = None) -> Thread:
    """A thread on this chain: the message to the actor, which answers it with a value of the shape."""
    words = (named(shape), message, to or "")
    found = self.life.again("prompt", self.id, words)
    return Thread(self, found or engine.prompt(shape, message, to=to or "", on=self.id))

  def chain(self, label: str, *, model: str | None = None) -> Chain:
    """A new chain on this one, whose prompts go to the model named, or to the default actor."""
    made = Chain(self.life, self.life.again("chain", self.id, (label, "")) or engine.chain(label, on=self.id))
    if model is not None:
      word = f"actor = {model!r}"
      self.life.again("rung", made.id, (word, "", "")) or engine.rung(word, on=made.id)
    return made

  def grant(self, *, usd: float | None = None, share: float | None = None) -> None:
    """A ceiling on this chain, in dollars or in a share of the window, which pauses the chain when it reaches it."""
    self.life.again("grant", self.id, (usd, share)) or engine.grant(usd=usd, share=share, on=self.id)

  def is_paused(self) -> bool:
    """Whether a pause holds this chain."""
    return bool(engine.paused(self.id))

  async def paused(self) -> None:
    """Wait until a pause holds this chain, as a grant says one at its ceiling."""
    await self.life.until(self.is_paused)

  def wake(self) -> None:
    """Wake this chain, so its work goes on."""
    engine.wake(self.id)

  def chains(self) -> list[Chain]:
    """Every chain under this one, each before the chains under it."""
    found = []
    for one in engine.transcript(self.id):
      if one[0] == "chain" and question("chain", one[1]) and one[3] == self.id:
        child = Chain(self.life, one[1])
        found += [child, *child.chains()]
    return found

  def acts(self) -> list[LifeAct]:
    """Every act on this chain, in order, with named fields."""
    return [
      LifeAct(one[0], one[1], one[2], one[3], tuple(one[4:]), engine.peek(one[1], ...) is not ..., engine.peek(one[1]))
      for one in engine.transcript(self.id)
      if question(one[0], one[1]) and one[3] == self.id
    ]


class Thread(str):
  """One prompt of a chain, which a program awaits for its value: the name of the prompt, and the door of its
  ladder, whose plan is the words of the models that answer it."""

  chain: Chain

  def __new__(cls, chain: Chain, id: str) -> Thread:
    made = super().__new__(cls, id)
    made.chain = chain
    return made

  def __await__(self) -> Generator[object, None, object]:
    return engine.Act(str(self)).__await__()

  @property
  def done(self) -> bool:
    """Whether the thread has its value."""
    return engine.peek(str(self), ...) is not ...

  @property
  def plan(self) -> str:
    """The words of the rungs of this thread, as python, in order."""
    words = engine.program(self.chain.id)
    return "\n".join(str(word) for whose, word in words.items() if engine.under(whose, str(self)))

  async def step(self) -> None:
    """Wait until the plan of the thread changes, as it does when a model answers it with a word."""
    before = self.plan
    await self.chain.life.until(lambda: self.done or self.plan != before)

  def edit(self, plan: str) -> None:
    """Write the door of the thread with a new plan: the chain cancels each word that the plan drops, and runs the
    new words."""
    engine.write(engine.Text(str(self), plan), on=self.chain.id)


@dataclass
class Question:
  """One prompt to the operator, which the program answers."""

  life: Life
  id: str
  chain: Chain
  shape: str
  message: str

  def answer(self, value: object) -> None:
    """Answer the question with a value of its shape, or with a line that the console reads as one."""
    if isinstance(value, str) and self.shape != "str":
      value = _monty.answered(self.shape, value)
    engine.close(value, self.id)


class Life(Chain):
  """One life of the engine: the root chain, the questions to the operator, and the ears that serve it."""

  def __init__(self, ears: dict[str, _monty.NativeEar], record: list[tuple]) -> None:
    self.ears, self.asked, self.waiting = ears, list[Question](), list[asyncio.Future[None]]()
    self.ended = False
    self.loop = asyncio.get_running_loop()
    self.made = [one for (one,) in record if one[2] == OPERATOR and one[0] in MADE and question(one[0], one[1])]
    self.made = [one for one in self.made if one[1] != "chain1"]
    root = engine.boot(record, console=self.console(), **ears)
    super().__init__(self, str(root))
    for chain in dict.fromkeys(engine.scope(id) for id, _ in engine.pending()):
      engine.wake(chain)

  def again(self, kind: str, on: str, words: tuple) -> str | None:
    """The act of the record that a program made at this place, when it agrees with this one."""
    if self.made and (one := self.made[0])[0] == kind and one[3] == on and tuple(one[4:]) == words:
      self.made.pop(0)
      return str(one[1])
    return None

  def console(self) -> Generator[tuple | None, tuple]:
    """The ear that takes each prompt to the operator as a question, and tells each waiter that the life moved."""
    while True:
      fact = yield
      self.loop.call_soon_threadsafe(self.moved)
      match fact:
        case ("prompt", about, _, on, shape, message, "operator") if question("prompt", about):
          self.asked.append(Question(self, about, Chain(self, on), shape, message))
          yield "started", about

  def moved(self) -> None:
    for one in self.waiting:
      if not one.done():
        one.set_result(None)
    self.waiting.clear()

  async def until(self, holds: Callable[[], bool]) -> None:
    """Wait until a condition of the life holds, which the life checks again after each fact, or raise once the life
    ended, since nothing of it moves again."""
    while not holds():
      if self.ended:
        raise RuntimeError(ENDED)
      one = self.loop.create_future()
      self.waiting.append(one)
      await one

  async def questions(self, until: Thread | None = None) -> AsyncIterator[Question]:
    """The questions to the operator, as the models ask them, until the thread named is done."""
    while until is None or not until.done:
      if self.asked:
        yield self.asked.pop(0)
        continue
      await self.until(lambda: bool(self.asked) or (until is not None and until.done))

  def end(self) -> None:
    """End the life: its commands, its waits and its models end, the store lets its record go, and each wait of the
    program on it raises that the life ended."""
    self.ended = True
    for one in self.ears.values():
      one.dispose()
    self.moved()


async def open(
  record: str | Path | None = None,
  *,
  model: str | None = None,
  directory: str | Path = ".",
  roster: list[str] | None = None,
  claude: str | None = None,
  extensions: bool = True,
) -> Life:
  """A life on its record, or a new one when the record holds nothing, whose prompts go to the model named, or to the
  default actor of the crate."""
  stored, ears = _monty.opened(
    directory=str(Path(directory).absolute()),
    record=None if record is None else str(record),
    actor=model,
    roster=roster,
    claude=claude,
    extensions=extensions,
  )
  life = Life(dict(ears), [(tuple(one),) for (one,) in stored])
  atexit.register(life.end)
  return life
