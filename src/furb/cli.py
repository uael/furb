"""The command line of furb: the door of the operator onto the root chain of one life.

One command is one life. The life is opened on a loop of its own, from the record it is given, and the operator
waits on that loop for what it asked. A prompt the record already holds is taken up again and never asked twice,
so a command said again on a kept record reads the answer of the life before it and asks no model for it.
"""

import argparse
import asyncio
import sys
from collections.abc import Sequence
from pathlib import Path

import furb_monty
from furb import engine, python, sheet
from furb.engine import Act
from furb.kernel import kernel
from furb.provider.claude import ACTOR, cool
from furb.world import Live, entries, kept
from furb_monty import _monty

SHAPES: dict[str, type | None] = {"none": None, "str": str, "int": int, "float": float, "bool": bool, "list": list}
"""SHAPES is every shape a prompt of the command line takes, by the name it is given on the line."""


def say(text: str) -> None:
  """One line to the operator, said as it happens, since a line of a long run that waits for the end says nothing."""
  sys.stdout.write(f"{text}\n")
  sys.stdout.flush()


def lived(record: Path | None, cwd: Path, actor: str, *, keeps: bool) -> tuple[Live, str, list[tuple]]:
  """One life on the loop that runs: its World, the ears of the crate, the Kernel, the gate of the crate, and its root.

  The life is made again from what the record holds, and it keeps what it says to the record when it keeps, through
  the store of the crate, which holds the lease of the record until the World ends. A life that only reads a record
  keeps nothing, since every life stands as it opens, and a life that keeps keeps that stand. The journal says the
  whole record again before boot returns, so the life stands whole on its record when this gives the root.
  """
  world = Live(str(cwd.absolute()), actor)
  world.ears = {"files": _monty.files(), "bash": _monty.bash(), "time": _monty.time()}
  if keeps and record is not None:
    record.parent.mkdir(parents=True, exist_ok=True)
    stored, world.ears["store"] = _monty.store(str(record))
    held = entries(stored)
  else:
    held = kept(record) if record is not None and record.is_file() else []
  gate = sheet.gating(vars(python), furb_monty.gate)
  root = engine.boot(held, world=world.hears(), kernel=kernel(vars(python)), gate=gate, **world.ears)
  return world, root, held


def again(held: Sequence[tuple], root: str, shape: type | None, message: str, to: str) -> str:
  """The name of the prompt the record already holds for this message, and nothing when it holds none.

  The engine matches nothing the operator says again, so a life stood up on its own record would open a second
  prompt beside the one that record stands on, and ask a model for what it was answered once.
  """
  named = shape.__name__ if isinstance(shape, type) else repr(shape)
  for entry in held:
    match entry:
      case (("prompt", id, "operator", on, kind, said, who),) if (on, kind, said, who) == (root, named, message, to):
        return id
  return ""


async def prompted(record: Path | None, cwd: Path, shape: type | None, message: str, to: str) -> object:
  """One prompt of the operator on the root of a life, awaited for the shape it asks for."""
  world, root, held = lived(record, cwd, ACTOR, keeps=True)
  try:
    if name := again(held, root, shape, message, to):
      return await Act(name)
    return await engine.prompt(shape, message, to, on=root)
  finally:
    # Nothing the life warmed outlives the life, so every process of the provider dies with the command.
    world.end()
    await cool()


async def turned(record: Path, cwd: Path, actor: str = ACTOR) -> None:
  """The turns of the root of a life made again from its record, on the actor that life stood on, each as the
  python a model reads of it."""
  world, root, _ = lived(record, cwd, actor, keeps=False)
  try:
    for role, py, _, _ in engine.turns(on=root):
      say(f"[{role}] {py}")
  finally:
    world.end()
    await cool()


async def running(record: Path | None, cwd: Path, word: str) -> object:
  """One word its caller wrote, run as a rung on the root of a life, awaited for what the word gave."""
  world, root, _ = lived(record, cwd, ACTOR, keeps=True)
  try:
    return await engine.rung(word, on=root)
  finally:
    world.end()
    await cool()


def parser() -> argparse.ArgumentParser:
  """What the command line takes: a prompt, the turns of a record, and a word the caller wrote."""
  whole = argparse.ArgumentParser(prog="furb", description="The core of an AI harness.")
  verbs = whole.add_subparsers(dest="verb", required=True)
  one = verbs.add_parser("prompt", help="Prompt an actor on the root chain and print what the prompt gave.")
  one.add_argument("message", help="What the actor is told.")
  one.add_argument("--to", default="", help="The actor, as model/effort; the default actor of the chain when unsaid.")
  one.add_argument("--shape", default="none", choices=sorted(SHAPES), help="The shape of the response.")
  one.add_argument("--record", type=Path, default=None, help="The record to keep, and to resume from.")
  one.add_argument("--cwd", type=Path, default=Path(), help="The directory the chains of the life start in.")
  two = verbs.add_parser("turns", help="Print the turns of the root of a life made again from its record.")
  two.add_argument("--record", type=Path, required=True, help="The record to resume from.")
  two.add_argument("--cwd", type=Path, default=Path(), help="The directory the chains of the life start in.")
  three = verbs.add_parser("run", help="Run a word the caller wrote on the root chain and print what it gave.")
  three.add_argument("word", help="The python of the word.")
  three.add_argument("--record", type=Path, default=None, help="The record to keep, and to resume from.")
  three.add_argument("--cwd", type=Path, default=Path(), help="The directory the chains of the life start in.")
  return whole


def main() -> None:
  """The console script: one command of the operator, on one life, on a loop of its own."""
  args = parser().parse_args()
  if args.verb == "prompt":
    say(repr(asyncio.run(prompted(args.record, args.cwd, SHAPES[args.shape], args.message, args.to))))
  elif args.verb == "turns":
    asyncio.run(turned(args.record, args.cwd))
  else:
    say(repr(asyncio.run(running(args.record, args.cwd, args.word))))
