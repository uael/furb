"""The doubles the modules around the engine are driven by: a model that answers from a script, a World, and a gate.

A test of the World stands its life on the World under test and on a gate that finds nothing; a test of the Kernel
stands its life on the World in memory of the suite. Neither asks a model.

A model of the suite answers with a coroutine, so the provider awaits it on the loop of the life. A plain function
would be run on a thread, and what it answers would land after a time of the machine and not after a count of turns
of the loop, so a test that gives the loop room would pass or fail by the load of the machine.
"""

import math
import os
import sys
from collections.abc import Generator, Iterator, Sequence
from contextlib import contextmanager

from pydantic_ai.messages import ModelMessage, ModelResponse, TextPart
from pydantic_ai.models.function import AgentInfo, FunctionModel

import conftest
from furb import engine
from furb.world import Live

type Words = Generator[tuple | None, tuple]
"""The World, an Ear of engine.pyi: engine.py binds no such name, so the suite says the type itself."""


def watched(seen: list[list[ModelMessage]], words: Sequence[str] = ()) -> FunctionModel:
  """A model that answers each ask with the next word of a script, and with close(None) once the script runs out,
  and keeps every list of messages it was handed, so a test reads what the World built."""
  said = list(words)

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del info
    seen.append(list(messages))
    return ModelResponse(parts=[TextPart(said.pop(0) if said else "close(None)")])

  return FunctionModel(turn)


def scripted(words: Sequence[str]) -> FunctionModel:
  """A model that answers each ask with the next word of a script, and that no test watches."""
  return watched([], words)


def mute(times: float = 1) -> FunctionModel:
  """A model that answers nothing for its first asks and answers close(3) after them, so one ask of nothing is a
  fault of the moment."""

  async def turn(messages: list[ModelMessage], info: AgentInfo) -> ModelResponse:
    del messages, info
    nonlocal times
    if times > 0:
      times -= 1
      raise RuntimeError("the model was not there")
    return ModelResponse(parts=[TextPart("close(3)")])

  return FunctionModel(turn)


def broken() -> FunctionModel:
  """A model that answers nothing at all, which is what an ask the World cannot answer looks like."""
  return mute(math.inf)


def worlds(stands: list) -> Words:
  """The World in memory of the suite on a standing: it answers the stand and takes a wait, which is what a test of
  the Kernel stands a life on, since the Kernel neither reads a disk nor asks a model."""
  return conftest.Sand(stands=stands).hears()


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


def booted(said: Words, record: Sequence[tuple] = (), *, gated: bool = True, **ears: Words) -> str:
  """A life on a World and these ears, with the Kernel of this interpreter and its gate when it is gated, and the id
  of its root; a life that is not gated reads every word with a gate that finds nothing, so it refuses no word."""
  py = conftest.Py()
  return engine.boot(record, kernel=py.kernel(), world=said, gate=py.gating() if gated else blind(), **ears)


def life(world: Live, record: Sequence[tuple] = ()) -> str:
  """A life on the World under test, which is not gated, since reading every word with ty would spend a second of
  the suite on each of them, so it is driven by the words a model would write."""
  return booted(world.hears(), record, gated=False)


async def settle() -> None:
  """Room for the loop to do what it still owes, so that finding nothing done means something.

  A turn of a model goes through the provider, which owes a number of turns of the loop that it alone knows, and
  the most that was ever counted here is two hundred. The room is ten times that, since a turn of the loop that
  nothing owes costs nothing and a count that is only just enough is a test that fails on another machine.
  """
  await conftest.settle(2000)


def heads(root: str) -> list[str]:
  """The header of every paragraph in the user turns of a chain, which is the first line of each."""
  return conftest.heads(engine.turns(on=root))
