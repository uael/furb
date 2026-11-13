"""The doubles the modules around the engine are driven by: a model that answers from a script, and a World.

A test of the World stands its life on the World under test; a test of the Kernel stands its life on the World in
memory of the suite. None asks a model: a model of the suite is a function that the provider of the crate asks in place of a model, on a thread
of its own, so a test waits for what it answers.
"""

import asyncio
import math
import os
import sys
from collections.abc import Callable, Iterator, Sequence
from contextlib import contextmanager
from pathlib import Path

import conftest
from conftest import World
from furb import engine
from furb.world import Answer, Live

ROSTER = ["claude-cli:opus"]
"""ROSTER is the roster of a World of the suite: one model, whose requests a double answers."""


def turn(word: str) -> tuple:
  """The turn of a model that wrote a word, with no usage and no blocks of a provider."""
  return ("assistant", word, None, None)


def watched(seen: list[dict], words: Sequence[str] = ()) -> Answer:
  """A model that answers each request with the next word of a script, and with close(None) once the script runs out,
  and keeps every request it was handed, so a test reads what the provider asked."""
  said = list(words)

  def answer(request: dict, write: Callable[..., None]) -> tuple:
    del write
    seen.append(request)
    return turn(said.pop(0) if said else "close(None)")

  return answer


def scripted(words: Sequence[str]) -> Answer:
  """A model that answers each request with the next word of a script, and that no test watches."""
  return watched([], words)


def mute(times: float = 1) -> Answer:
  """A model that answers nothing for its first requests and answers close(3) after them, so one request of nothing
  is a fault of the moment."""

  def answer(request: dict, write: Callable[..., None]) -> tuple:
    del request, write
    nonlocal times
    if times > 0:
      times -= 1
      why = "the model was not there"
      raise RuntimeError(why)
    return turn("close(3)")

  return answer


def broken() -> Answer:
  """A model that answers nothing at all, which is what a request the provider cannot answer looks like."""
  return mute(math.inf)


def world(yard: object, answer: Answer | None = None) -> Live:
  """The World of this machine, in the directory of the test, on a model that a double answers."""
  return Live(str(yard), roster=ROSTER, answer=answer)


def worlds(stands: list) -> World:
  """The World in memory of the suite on a standing: it answers the stand and takes a wait, which is what a test of
  the Kernel stands a life on, since the Kernel neither reads a disk nor asks a model."""
  return conftest.Sand(stands=stands).hears()


@contextmanager
def speaking(text: str) -> Iterator[None]:
  """The operator at its terminal, saying this text and no more, which the World reads on the loop it runs on."""
  read, wrote = os.pipe()
  os.write(wrote, text.encode())
  os.close(wrote)
  held, sys.stdin = sys.stdin, os.fdopen(read)
  try:
    yield
  finally:
    sys.stdin.close()
    sys.stdin = held


def booted(said: World, record: Sequence[tuple] = (), **ears: World) -> str:
  """A life on a World and these ears, with the Kernel of this interpreter, and the id of its root."""
  return engine.boot(record, world=said, **ears, kernel=conftest.Py().kernel())


def life(live: Live, record: Path | None = None) -> str:
  """A life on the World under test and on the ears of the crate, its provider among them, which enables no extension,
  so it is driven by the words a model would write."""
  return booted(live.hears(), live.opened(record, skills=False), **live.ears)


async def settle() -> None:
  """Room for the loop to do what it still owes, so that finding nothing done means something."""
  await conftest.settle(2000)


async def until(holds: Callable[[], bool]) -> bool:
  """Whether a condition holds within ten seconds, since a model of the suite answers from a thread of its own."""
  for _ in range(1000):
    if holds():
      return True
    await asyncio.sleep(0.01)
  return holds()
