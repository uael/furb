"""The command line of furb: the door of the operator onto the root chain of one life.

One command is one life. The life is opened on a loop of its own, from the record it is given, and the operator
waits on that loop for what it asked. A prompt the record already holds is taken up again and never asked twice,
so a command said again on a kept record reads the answer of the life before it and asks no model for it.

The life takes the extensions the configs name, as the crate reads them: the builtins and what the config of the
user and the config of the project add. The module of the engine runs the word of each after the engine, or the
words the record pins, the life plays their life words, and the command line holds the part for a World of each
builtin; it loads no part for a World of another extension.
"""

import argparse
import asyncio
import sys
from collections.abc import Sequence
from pathlib import Path

import furb_monty
from furb import engine
from furb.engine import WORLD, Act, Refused
from furb.kernel import Native, extended, gating
from furb.provider.claude import ACTOR, cool
from furb.world import Live, kept

SHAPES: dict[str, type | None] = {"none": None, "str": str, "int": int, "float": float, "bool": bool, "list": list}
"""SHAPES is every shape a prompt of the command line takes, by the name it is given on the line."""


def say(text: str) -> None:
  """One line to the operator, said as it happens, since a line of a long run that waits for the end says nothing."""
  sys.stdout.write(f"{text}\n")
  sys.stdout.flush()


def lived(record: Path | None, cwd: Path, actor: str, *, keeps: bool) -> tuple[Live, str, list[tuple]]:
  """One life on the loop that runs: its World on the record, the Kernel of this interpreter, and its root.

  The life is made again from what the record holds, and it keeps what it says to the record when it keeps. A
  life that only reads a record keeps nothing, since a World given the record it reads appends to it: a stand of a
  directory or an actor other than the one the record holds is a stood it keeps. The journal says the whole record
  again before boot returns, so the life stands whole on its record when this gives the root.
  """
  held = kept(record) if record is not None and record.is_file() else []
  loaded = furb_monty.extensions(str(cwd.absolute()))
  pinned = furb_monty.pinned_words(held)
  words = [one.word for one in loaded if one.word] if pinned is None else pinned
  lives = [one.life for one in loaded if one.life]
  parts = tuple(one.name for one in loaded)
  world = Live(str(cwd.absolute()), record if keeps else None, actor, words=words, lives=lives, parts=parts)
  source = extended(words)
  root = engine.boot(held, world=world.hears(), kernel=Native().kernel(), gate=gating(source))
  if pinned is None and words:
    engine.send("extensions", root, words, by=WORLD)
  world.play()
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
  _, root, held = lived(record, cwd, ACTOR, keeps=True)
  try:
    if name := again(held, root, shape, message, to):
      return await Act(name)
    return await engine.prompt(shape, message, to, on=root)
  finally:
    # Nothing the life warmed outlives the life, so every process of the provider dies with the command.
    await cool()


async def turned(record: Path, cwd: Path) -> None:
  """The turns of the root of a life made again from its record, each as the python a model reads of it."""
  root = lived(record, cwd, ACTOR, keeps=False)[1]
  try:
    for role, py, _, _ in engine.turns(on=root):
      say(f"[{role}] {py}")
  finally:
    await cool()


async def running(record: Path | None, cwd: Path, word: str) -> object:
  """One word its caller wrote, run as a rung on the root of a life, awaited for what the word gave."""
  root = lived(record, cwd, ACTOR, keeps=True)[1]
  try:
    return await engine.rung(word, on=root)
  finally:
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
  four = verbs.add_parser("update", help="Fetch the extensions the configs name again, and print their names.")
  four.add_argument("--cwd", type=Path, default=Path(), help="The directory of the project whose config is read.")
  return whole


def main() -> None:
  """The console script: one command of the operator, on one life, on a loop of its own. A config, a fetch or a
  manifest of an extension that fails ends the command with what failed."""
  args = parser().parse_args()
  try:
    if args.verb == "prompt":
      say(repr(asyncio.run(prompted(args.record, args.cwd, SHAPES[args.shape], args.message, args.to))))
    elif args.verb == "turns":
      asyncio.run(turned(args.record, args.cwd))
    elif args.verb == "update":
      for one in furb_monty.extensions(str(args.cwd.absolute()), refresh=True):
        say(one.name)
    else:
      say(repr(asyncio.run(running(args.record, args.cwd, args.word))))
  except Refused as no:
    why = f"furb: {no}"
    raise SystemExit(why) from None
