"""The play: a life with its whole runtime, and a model asked to use every part of it.

Run it from the root of the repository, as `sleep 3000 | uv run python -u script/play.py`. The stdin of the play
must stay open and say nothing: the World shows a prompt of the operator on the terminal and reads a line back,
and this play answers that prompt from the side of the operator instead, after a settle.

One life does the work and a second life is opened on the record it kept, which answers every rung of the first
without asking a model, and then takes one prompt more.
"""

import asyncio
import os
import shutil
import sys
import tempfile
import time
from pathlib import Path

from furb import engine
from furb.cli import lived, say
from furb.engine import OPERATOR
from furb.provider.claude import BIN, cool
from furb.world import kept, rendered

TO = "opus/low"
"""TO is the actor the play asks, which is opus at the least effort it takes."""
CEILING = 4.0
"""CEILING is the dollars the play may spend, on each chain it sees and over all of them together."""
QUIET = 240.0
"""QUIET is the seconds the record may stand still before the play reads the chain as wedged and ends the prompt."""
TRIES = 40
"""TRIES is the answers the play buys before it reads the model as unable to answer and ends the prompt."""
STALL = 1800.0
"""STALL is the seconds the play waits for one prompt, since a chain that went quiet would wait for ever."""
SETTLE = 2.0
"""SETTLE is the seconds the operator waits between two looks at the transcript, since each look is a read."""
SAID = "windlass"
"""SAID is the word the operator answers the model with, which the model holds and gives back."""
ITEMS = 7
"""ITEMS is how many things the model is asked to find, which is the length of the list it closes with."""

MESSAGE = f"""You have the engine and a directory of your own. Use them, in this order.

1. Read the engine at {Path(engine.__file__)} and say in one line what a chain is.
2. Write a small python module into your directory, run it with bash, and read its stdout.
3. Open a chain with your own chain as its source and take() as its filter, so the model you ask on it reads
   none of your work, and end that word without closing anything. In the next word, prompt opus/low on that chain
   with shape int for a small sum, and await the value. Say in that message that the answer is a close of the number.
4. Prompt the operator, which is the actor named "operator", with shape str for a word, await it, and hold the
   word it gives back.
5. Debug one value with a template string, and peek at one act you made.
6. Make a subdirectory, cd into it by its full path, and show that cwd changed.
7. Close with a list of exactly {ITEMS} items, in this order: your one line about a chain (str), the stdout of your
   module (str), the int the chain you opened gave (int), the word of the operator (str), the id of the act you
   peeked at (str), the working directory you are now in (str), and the id of the chain you opened (str).
"""
"""MESSAGE is what the model is told: what it has, what to do with it, and what to give back."""


def tags(root: str, name: str) -> list[tuple]:
  """Every tag of that name in the turns of a chain."""
  return [
    one for _, content, _, _ in engine.turns(on=root) for one in content if isinstance(one, tuple) and one[0] == name
  ]


def transcript(root: str) -> list[tuple]:
  """The transcript of a chain, which the operator asks of the chain itself."""
  _, got = engine.ask("transcript", root, root)
  assert isinstance(got, list), got
  return got


async def settle(n: int = 400) -> None:
  """Room for the loop to do what it still owes, so that finding nothing done means something."""
  for _ in range(n):
    await asyncio.sleep(0)


def answered(record: Path) -> list[tuple]:
  """Every answer of a model that the record holds."""
  said = kept(record) if record.is_file() else []
  return [entry[1] for entry in said if entry[1][0] == "answer" and entry[1][3] is not None]


def spent(record: Path) -> float:
  """What every answer the record holds has cost so far."""
  return sum(one[3][2][4] for one in answered(record) if one[3][2])


async def watching(root: str, record: Path, name: str) -> None:
  """The operator while the model works.

  It answers the one prompt the model puts to it, and it ends the prompt when the spend of the whole life crosses
  the ceiling, since a grant is of one chain alone and a prompt of the model opens chains of its own. It puts no
  grant on those chains: a grant of the operator on a chain that a rung opened cannot be made again, since a later
  life says the acts of the operator at once and that chain does not stand yet.
  """
  size, still = 0, time.monotonic()
  while True:
    await asyncio.sleep(SETTLE)
    for one in transcript(root):
      match one:
        case ("prompt", pid, _, _, _, _, "operator") if engine.peek(pid, on=root) is None:
          engine.close(SAID, pid)
    if spent(record) > CEILING:
      say(f"the play spent {spent(record):.4f} dollars, over its ceiling of {CEILING}, and ends the prompt")
      engine.cancel(name)
      return
    if len(answered(record)) > TRIES:
      say(f"the play bought {len(answered(record))} answers, over the {TRIES} it allows, and ends the prompt")
      engine.cancel(name)
      return
    grown = record.stat().st_size if record.is_file() else 0
    if grown != size:
      size, still = grown, time.monotonic()
    elif time.monotonic() - still > QUIET:
      say(f"nothing entered the record for {QUIET:.0f} seconds, so the play reads the chain as wedged")
      engine.cancel(name)
      return


async def first(yard: Path, record: Path) -> list[object]:
  """The life that does the work, and everything the play holds it to when the work is done."""
  _world, root, held = lived(record, yard, TO)
  assert held == [], "the first life is opened on no record"
  engine.grant(usd=CEILING, on=root)
  waits = engine.prompt(list, MESSAGE, TO, on=root)
  watched = asyncio.ensure_future(watching(root, record, waits))
  try:
    got = await asyncio.wait_for(waits, STALL)
  finally:
    watched.cancel()
  assert isinstance(got, list), got
  assert len(got) == ITEMS, got
  said = transcript(root)

  forks = [one for one in said if one[0] == "chain" and one[5] == root]
  assert forks, "the model opened no chain with a source of its own"
  assert got[6] in {one[1] for one in forks}, (got[6], [one[1] for one in forks])
  # The chain it names must have served it: a word of a model ran on it, and a prompt of the model stands there.
  assert isinstance(got[6], str), got[6]
  theirs = transcript(got[6])
  assert [one for one in theirs if one[0] == "prompt"], f"no prompt of the model stands on {got[6]}"
  assert [one for one in theirs if one[0] == "answer"], f"no model answered on {got[6]}"

  asked = [one for one in said if one[0] == "prompt" and one[6] == OPERATOR]
  assert len(asked) == 1, f"the model put {len(asked)} prompts to the operator"
  assert engine.peek(asked[0][1], on=root) == SAID, engine.peek(asked[0][1], on=root)
  assert got[3] == SAID, got[3]

  made = sorted(one.name for one in yard.rglob("*.py"))
  assert made, "the model wrote no python module to its directory"
  assert isinstance(got[1], str), got[1]
  assert got[1].strip(), f"the module of the model said {got[1]!r}"

  assert isinstance(got[0], str), got[0]
  assert got[0].strip(), got[0]
  assert isinstance(got[2], int), got[2]
  assert tags(root, "debugged"), "no debug of the model stands in the turns"
  assert tags(root, "peek"), "no peek of the model stands in the turns"
  assert got[4] in {one[1] for one in said if engine.question(one)}, got[4]

  assert got[5] != str(yard), f"the working directory of the chain did not move from {yard}"
  assert engine.cwd(on=root) == got[5], (engine.cwd(on=root), got[5])

  say(f"the model wrote {made} into its directory")
  say(f"the chain it opened is {got[6]}, with a source of {root}, of {len(forks)} it opened")
  say(f"the operator was asked {asked[0][5]!r} and answered {SAID!r}")
  say(f"the working directory moved to {got[5]}")
  say(f"the first life gave {got!r}")
  await cool()
  return got


async def second(yard: Path, record: Path, got: list[object]) -> float:
  """The life on the record of the first: it asks no model for what the record holds, and takes one prompt more."""
  world, root, held = lived(record, yard, TO)
  await settle()
  asks = [one for one in world.calls if one[0] == "ask"]
  assert asks == [], f"the resumed life asked a model {len(asks)} times for what its record holds"
  assert held, "the resumed life was opened on nothing"
  say(f"the resumed life made {len(held)} words of its record again and asked no model")
  # A pause the World said when it could not reach a model stands in the record, so every later life of that record
  # opens paused. The World tells the operator why it went quiet, and waking it again is the operator's to do.
  if [entry for entry in held if entry[1][0] == "pause" and entry[1][1] == root]:
    say("the record holds a pause of the root, said by the World, so the operator wakes the chain")
    engine.wake(root)
    await settle()
  message = "How many items did you give back? Close with the number and nothing else."
  try:
    more = await asyncio.wait_for(engine.prompt(int, message, TO, on=root), STALL)
    assert more == len(got), (more, len(got))
    say(f"the resumed life answered {more!r} for the length of that list")
    return ledger(root, record)
  finally:
    await cool()


def ledger(root: str, record: Path) -> float:
  """Everything the play shows: the turns as the model read them, the words it wrote, and what they cost."""
  say("")
  say("=== the turns of the root, as the model read them ===")
  for role, content, _, _ in engine.turns(on=root):
    say(f"[{role}] {rendered(content)}")
  say("")
  say("=== the program of the root: every word the model wrote that the gate took ===")
  _, program = engine.ask("program", root)
  assert isinstance(program, dict), program
  for name, word in program.items():
    say(f"--- {name}")
    say(str(word))
  say("")
  say("=== the ledger ===")
  for one in answered(record):
    if one[3][2] is not None:
      say(f"{one[1]} {one[3][2]}")
  say(f"the play spent {spent(record):.6f} dollars over {len(kept(record))} entries of record")
  return spent(record)


def main() -> int:
  """The two lives, in a directory of their own, and everything the play has to show for them."""
  if shutil.which(os.environ.get(BIN) or "claude") is None:
    say("no claude on PATH, so no model can be asked and the play stops here.")
    return 1
  yard = Path(tempfile.mkdtemp(prefix="furb-play-"))
  record = yard / "record.jsonl"
  say(f"the directory of the play is {yard}")
  got = asyncio.run(first(yard, record))
  asyncio.run(second(yard, record, got))
  say("PLAY OK")
  return 0


if __name__ == "__main__":
  sys.exit(main())
