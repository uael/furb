"""The bench of compaction: compacted on a copy of a real record, measured before and after.

Run it from the root of the repository on a copy of a record, never on the original:

    uv run python script/bench/compaction.py <copy of a record> <chain> [--cut <chain>] [--dry]

The bench opens a life on the copy, on the engine of this checkout. A record that another engine made drifts, so the
bench writes the name of this engine into the copy, and `--cut` keeps of the copy only the lines before the first
line that names that chain, where the replay of the copy would drift. The life runs no command, writes nothing and
asks the operator nothing. It asks a model only the choice and the first answer after the cut, both on the chain.

On the chain it is given, the bench plays the word of the official extension compaction, wraps the chain with
compacted() at a SHARE of -1, and pokes the chain twice: an answer of the bench closes each poke, so compacted asks
choice() of the model of the chain, on the chain, after the first, and cuts the chain in place at the second. Then it
waits for the first answer of a model on the chain after the cut, unless `--dry` stops it at the cut. It prints the
share of the window of the last answer before and of the first answer after, the tokens of `o200k_base` in the turns
before and after, the choice, the acts kept, and the first word of the model after the cut.
"""

import argparse
import asyncio
import json
import sys
import time
from collections.abc import Callable, Generator
from functools import partial
from pathlib import Path

import tiktoken

from furb import engine, python
from furb.engine import OPERATOR, Refused
from furb.kernel import kernel
from furb.world import Live
from furb_monty import _monty

POKE = "poke: a measurement asks nothing here, close(None)"
"""POKE is the message of each poke, which an answer of the bench closes."""
ENCODING = tiktoken.get_encoding("o200k_base")
"""ENCODING counts the tokens of the turns, the same on each machine."""
POKED = 1000
"""POKED is the tokens that an answer of the bench to a poke says it read."""


async def engined(record: Path, cut: str) -> None:
  """The copy of a record with the name of this engine, and with no line from the first one that names the cut."""
  lines = record.read_text(encoding="utf-8").splitlines(keepends=True)
  if cut:
    lines = lines[: next((at for at, line in enumerate(lines) if f'"{cut}"' in line), len(lines))]
  fresh = record.with_suffix(".fresh")
  fresh.unlink(missing_ok=True)
  world = Live(str(record.parent), None)
  held = world.opened(fresh, keeps=True, skills=False)
  engine.boot(held, world=world.hears(), kernel=kernel(vars(python)), **world.ears)
  await asyncio.sleep(0.5)
  world.end()
  made = next(json.loads(one)[0] for one in fresh.read_text(encoding="utf-8").splitlines() if '"made"' in one)
  fresh.unlink()
  for at, line in enumerate(lines[:5]):
    fact = json.loads(line)
    if fact[0][0] == "made":
      fact[0][3:] = made[3:]
      lines[at] = json.dumps(fact) + "\n"
  record.write_text("".join(lines), encoding="utf-8")


def said(reply: str, turn: tuple) -> None:
  """The done of a reply that the bench answers, said as the World says it."""
  with engine.site.set("bench"):
    engine.say("done", reply, turn)


def bench() -> Generator[tuple | None, tuple | None]:
  """The ear of the bench: it refuses each command and each write, holds each prompt to the operator, closes each
  poke, and holds each reply but those of the choice of compacted and those after its cut, which the provider takes."""
  loop = asyncio.get_running_loop()
  while True:
    a = yield None
    if not a or not engine.question(a):
      continue
    if a[0] in ("bash", "write", "cd", "search", "fetch"):
      yield "done", a[1], Refused("the bench runs no command and writes nothing")
    elif a[0] == "prompt" and a[6] == OPERATOR:
      yield "started", a[1]
    elif a[0] == "reply":
      maker = engine.get(engine.get(a[2])[2]) or ("",)
      poke = maker[0] == "prompt" and maker[5] == POKE
      if not poke and (compacting(maker[2]) or any(compacting(x[2]) for x in engine.transcript(a[3], "rung"))):
        continue
      yield "started", a[1]
      if poke:
        loop.call_soon(partial(said, a[1], ("assistant", "close(None)", (POKED, 0, 0, 0, 0.0), [])))


def compacting(name: object) -> bool:
  """Whether a name is that of an act of compacted, which asks the choice and says the cut."""
  return (engine.get(str(name)) or ("",))[0] == "compacted"


def chosen(chain: str, watch: str) -> list[str]:
  """The prompts of the choice that the act watch of compacted made on the chain."""
  return acts(chain, "prompt", _2=watch)


def cut(chain: str, watch: str) -> bool:
  """Whether the act watch of compacted cut the chain in place: the cut makes the rung that binds the summary and
  regrounds the chain."""
  return any(a[2] == watch for a in engine.transcript(chain, "rung"))


def share(reply: str) -> float:
  """The share of the window of its actor that an answer read."""
  return engine.filled(reply) or 0.0


def tokens(chain: str) -> int:
  """The tokens of `o200k_base` in the text of the turns of a chain."""
  return sum(len(ENCODING.encode(str(turn[1]))) for turn in engine.turns(on=chain))


async def until(test: Callable[[], object], seconds: float) -> None:
  """A wait until the test holds, or until the seconds pass."""
  end = time.time() + seconds
  while not test() and time.time() < end:
    await asyncio.sleep(1)


def acts(chain: str, kind: str, **words: object) -> list[str]:
  """The acts of a kind on a chain, whose words at their places hold these values."""
  return [
    str(a[1])
    for a in engine.transcript(chain)
    if a[0] == kind and all(a[int(at[1:])] == one for at, one in words.items())
  ]


async def measured(record: Path, chain: str, *, dry: bool) -> dict[str, object]:
  """What the bench prints of one chain of a record."""
  world = Live(str(record.parent), None)
  held = world.opened(record, keeps=False, skills=False)
  engine.boot(held, bench=bench(), world=world.hears(), kernel=kernel(vars(python)), **world.ears)
  replies = [one for one in acts(chain, "reply") if engine.scope(one) == chain]
  out: dict[str, object] = {"chain": chain, "label": engine.get(chain)[4]}
  out |= {"share before": share(replies[-1]), "tokens before": tokens(chain)}
  word = next(str(one["word"]) for one in _monty.official() if one["name"] == "compaction")
  await engine.rung(word + "\nSHARE = -1.0\nwatch = compacted()", on=chain)
  watch = str(engine.module(chain)["watch"])
  await engine.prompt(None, POKE, on=chain)
  await until(lambda: chosen(chain, watch) and engine.over(chosen(chain, watch)[0]), 1200)
  out["choice"] = engine.peek(chosen(chain, watch)[0])
  held = len([a for a in engine.transcript(chain) if engine.question(a)])
  engine.prompt(None, POKE, on=chain)
  await until(lambda: cut(chain, watch), 60)
  out["kept acts"] = (len([a for a in engine.transcript(chain) if engine.question(a)]), held)
  out["tokens at the cut"] = tokens(chain)
  before = set(acts(chain, "reply"))
  if not dry:

    def answered() -> list[tuple]:
      return [
        got
        for one in acts(chain, "reply")
        if one not in before and isinstance(got := engine.peek(one), tuple) and got[2][0] != POKED
      ]

    await until(answered, 1500)
    first = next((one for one in acts(chain, "reply") if one not in before and engine.peek(one) in answered()), "")
    if first:
      out |= {"share after": share(first), "tokens after": tokens(chain), "word after": answered()[0][1]}
  world.end()
  return out


def main() -> None:
  """The bench, as the command line runs it."""
  parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
  parser.add_argument("record", type=Path, help="a copy of a record, which the bench changes")
  parser.add_argument("chain", help="the chain that compacted compacts")
  parser.add_argument("--cut", default="", help="a chain whose first line, and each line after it, the copy drops")
  parser.add_argument("--dry", action="store_true", help="stop at the birth of the new chain, and ask no model on it")
  given = parser.parse_args()

  async def both() -> dict[str, object]:
    await engined(given.record, given.cut)
    return await measured(given.record, given.chain, dry=given.dry)

  sys.stdout.write(json.dumps(asyncio.run(both()), indent=1, default=str) + "\n")


if __name__ == "__main__":
  main()
