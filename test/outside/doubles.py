"""The doubles the four modules are driven by: a model that answers from a script, a World, and a Kernel.

A test of the World stands its life on a Kernel that gates everything and runs nothing; a test of the Kernel stands
its life on a World that answers a standing and refuses every other question. Neither asks a model or reads a disk.
Every fact is a tuple, as the contract says: (kind, about, by, *words), and a question (kind, id, by, on, *words).
"""

import asyncio
import os
import sys
from collections.abc import Generator, Iterator, Sequence
from contextlib import contextmanager
from decimal import Decimal
from functools import partial

from pydantic_ai.messages import ModelMessage, ModelRequest, ModelResponse, TextPart, ThinkingPart, UserPromptPart
from pydantic_ai.models.function import AgentInfo, FunctionModel
from pydantic_ai.usage import RequestUsage

from furb import engine
from furb.engine import WORLD, Refused
from furb.kernel import Native
from furb.world import Live

type Words = Generator[tuple | None, tuple]
"""The World, as engine.pyi declares it: engine.py binds no such name, so the suite says the type itself."""


def scripted(words: Sequence[str], usd: float = 0.0) -> FunctionModel:
  """A model that answers each ask with the next word of a script, and with nothing once the script runs out."""
  said = list(words)

  def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    word = said.pop(0) if said else "close(None)"
    spent = RequestUsage(input_tokens=100, output_tokens=20, cost=Decimal(str(usd)))
    return ModelResponse(parts=[ThinkingPart(content="", signature="s"), TextPart(word)], usage=spent)

  return FunctionModel(turn)


def watched(seen: list[list[ModelMessage]], words: Sequence[str] = ()) -> FunctionModel:
  """A model that keeps every list of messages it was handed, so a test reads what the World built."""
  said = list(words)

  def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del info
    seen.append(list(messages))
    return ModelResponse(parts=[TextPart(said.pop(0) if said else "close(None)")])

  return FunctionModel(turn)


def ask(text: str) -> ModelRequest:
  """One message of the user, as pydantic_ai holds it."""
  return ModelRequest(parts=[UserPromptPart(content=text)])


def reply(text: str) -> ModelResponse:
  """One message of a model, as pydantic_ai holds it, with the emptied thinking part a record gives back."""
  return ModelResponse(parts=[ThinkingPart(content="", signature="sig"), TextPart(content=text)])


def worlds(stands: tuple) -> Words:
  """A World of the suite: it answers a stand, it does a wait, and it refuses every other question it is put.

  It is what a test of the Kernel stands a life on, since the Kernel neither reads a disk nor asks a model, and a
  word that awaits needs one act of the World that comes to something.
  """
  acts: dict[str, tuple] = {}
  loop = asyncio.get_running_loop()
  while True:
    a = yield
    match a:
      case (_, id, *_) if engine.question(a) and id in engine.acts:
        acts[id] = a
      case ("start", about, _):
        match acts.get(about):
          case ("wait", _, _, _, seconds):
            loop.call_later(seconds, partial(engine.send, "done", about, None, by=WORLD))
      case ("stand", qid, *_):
        yield "done", qid, stands
      case (kind, qid, *_) if engine.question(a) and qid in engine.asked:
        yield "done", qid, Refused(f"the suite answers no {kind}")


class Quick(Native):
  """A Kernel of the suite: the Kernel of this interpreter with its gate open.

  A World under test is no gate, and reading every word with ty would spend a second of the suite on each of them.
  What it runs it runs the one way the Kernel does, so a World is driven by the words a model would write.
  """

  def gate(self, word: str, ladder: list[str], shape: str) -> list[str]:
    """Nothing, since a World under test is held to what the words of a model do and not to what a gate says."""
    del word, ladder, shape
    return []


def broken(why: str = "the model was not there") -> FunctionModel:
  """A model that answers nothing at all, which is what an ask the World cannot answer looks like."""

  def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    raise RuntimeError(why)

  return FunctionModel(turn)


def mute(times: int = 1, word: str = "close(3)") -> FunctionModel:
  """A model that answers nothing for its first asks and answers after them, which is a fault of the moment."""

  def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    nonlocal times
    if times > 0:
      times -= 1
      raise RuntimeError("the model was not there")
    return ModelResponse(parts=[TextPart(word)])

  return FunctionModel(turn)


@contextmanager
def speaking(text: str) -> Iterator[None]:
  """The operator at its terminal, saying one line and no more, which the World reads on the loop it runs on."""
  read, wrote = os.pipe()
  os.write(wrote, text.encode())
  os.close(wrote)
  held, sys.stdin = sys.stdin, os.fdopen(read)
  try:
    yield
  finally:
    sys.stdin.close()
    sys.stdin = held


def stood(said: Words, *, gated: bool = True) -> str:
  """A life on a World of the suite, with the Kernel of this interpreter, and the id of its root."""
  return engine.boot((), kernel=(Native() if gated else Quick()).kernel(), world=said)


def life(world: Live, record: Sequence[tuple] = (), *, gated: bool = False) -> str:
  """A life on the World under test, with the Kernel of this interpreter, and the id of its root."""
  return engine.boot(record, kernel=(Native() if gated else Quick()).kernel(), world=world.hears())


async def settle(n: int = 2000) -> None:
  """Room for the loop to do what it still owes, so that finding nothing done means something.

  A turn of a model goes through the provider, which owes a number of turns of the loop that it alone knows, and
  the most that was ever counted here is two hundred. The room is ten times that, since a turn of the loop that
  nothing owes costs nothing and a count that is only just enough is a test that fails on another machine.
  """
  for _ in range(n):
    await asyncio.sleep(0)


def tags(root: str, name: str = "") -> list[tuple]:
  """Every tag of that name in the turns of a chain."""
  return [
    one
    for _, content, _, _ in engine.turns(on=root)
    for one in content
    if isinstance(one, tuple) and name in ("", one[0])
  ]
