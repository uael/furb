"""The doubles the four modules are driven by: a model that answers from a script, a World, and a Kernel.

A test of the World stands its life on a Kernel that gates everything and runs nothing; a test of the Kernel stands
its life on a World that answers a standing and refuses every other question. Neither asks a model or reads a disk.
Every fact is a tuple, as the contract says: (kind, about, by, *words), and a question (kind, id, by, on, *words).

A model of the suite answers with a coroutine, so the provider awaits it on the loop of the life. A plain function
would be run on a thread, and what it answers would land after a time of the machine and not after a count of turns
of the loop, so a test that gives the loop room would pass or fail by the load of the machine.
"""

import asyncio
import os
import re
import sys
from collections.abc import Generator, Iterator, Sequence
from contextlib import contextmanager
from decimal import Decimal
from functools import partial

from pydantic_ai.messages import ModelMessage, ModelRequest, ModelResponse, TextPart, ThinkingPart, UserPromptPart
from pydantic_ai.models.function import AgentInfo, FunctionModel
from pydantic_ai.usage import RequestUsage

from furb import engine
from furb.kernel import Native, gating
from furb.world import Live

type Words = Generator[tuple | None, tuple]
"""The World, an Ear of engine.pyi: engine.py binds no such name, so the suite says the type itself."""


def scripted(words: Sequence[str], usd: float = 0.0) -> FunctionModel:
  """A model that answers each ask with the next word of a script, and with nothing once the script runs out."""
  said = list(words)

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    word = said.pop(0) if said else "close(None)"
    spent = RequestUsage(input_tokens=100, output_tokens=20, cost=Decimal(str(usd)))
    return ModelResponse(parts=[ThinkingPart(content="", signature="s"), TextPart(word)], usage=spent)

  return FunctionModel(turn)


def watched(seen: list[list[ModelMessage]], words: Sequence[str] = ()) -> FunctionModel:
  """A model that keeps every list of messages it was handed, so a test reads what the World built."""
  said = list(words)

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
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


def worlds(stands: list) -> Words:
  """A World of the suite: it answers a stand, it takes a wait and says it done when the time is up, it takes a
  reply and never answers it, since no model stands behind it, and it takes no other question, so the life refuses
  each of them.

  It is what a test of the Kernel stands a life on, since the Kernel neither reads a disk nor asks a model, and a
  word that awaits needs one act of the World that comes to something.
  """
  loop = asyncio.get_running_loop()
  while True:
    match (yield):
      case ("wait", about, _, _, seconds):
        yield "started", about
        loop.call_later(seconds, partial(engine.say, "done", about, None))
      case ("reply", about, *_):
        yield "started", about
      case ("stand", qid, *_):
        yield "done", qid, stands


def broken(why: str = "the model was not there") -> FunctionModel:
  """A model that answers nothing at all, which is what an ask the World cannot answer looks like."""

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    raise RuntimeError(why)

  return FunctionModel(turn)


def mute(times: int = 1, word: str = "close(3)") -> FunctionModel:
  """A model that answers nothing for its first asks and answers after them, which is a fault of the moment."""

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
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


def blind() -> Words:
  """A gate that finds nothing in any word, so every word runs: the gate of a life whose test is not the gate."""
  while True:
    match (yield):
      case ("gate", qid, *_):
        yield "done", qid, []


def booted(said: Words, *, gated: bool = True) -> str:
  """A life on a World of the suite, with the Kernel of this interpreter and its gate when it is gated, and the id
  of its root; a life that is not gated reads every word with a gate that finds nothing, so it refuses no word."""
  return engine.boot((), kernel=Native().kernel(), world=said, gate=gating() if gated else blind())


def life(world: Live, record: Sequence[tuple] = (), *, gated: bool = False) -> str:
  """A life on the World under test, with the Kernel of this interpreter and its gate when it is gated, and the id
  of its root. The gate of a World under test finds nothing, since reading every word with ty would spend a second
  of the suite on each of them, so it is driven by the words a model would write."""
  return engine.boot(record, kernel=Native().kernel(), world=world.hears(), gate=gating() if gated else blind())


async def settle(n: int = 2000) -> None:
  """Room for the loop to do what it still owes, so that finding nothing done means something.

  A turn of a model goes through the provider, which owes a number of turns of the loop that it alone knows, and
  the most that was ever counted here is two hundred. The room is ten times that, since a turn of the loop that
  nothing owes costs nothing and a count that is only just enough is a test that fails on another machine.
  """
  for _ in range(n):
    await asyncio.sleep(0)


def heads(root: str) -> list[str]:
  """The header of every paragraph in the user turns of a chain, which is the first line of each."""
  return [
    one.split("\n", 1)[0]
    for role, py, _, _ in engine.turns(on=root)
    if role == "user" and py
    for one in re.split(r"\n\n(?=#\w)", py)
  ]
