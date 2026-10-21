"""furb as a framework, from python: a durable life, a tree of chains, a prompt to the operator, a grant and a door.

Run it as it stands, as `uv run python docs/examples/python.py`, on the engine of this interpreter, or with
`FURB_ENGINE=monty` on the engine of the crate. A scripted model answers in place of a real one, so it costs nothing.
"""

import asyncio
import json
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable, Generator
from pathlib import Path

import furb
import furb_monty
from furb import engine, python, sheet
from furb.engine import OPERATOR, Text
from furb.kernel import kernel
from furb.world import Live

PLAN = "Plan the release."
REVIEW = "Review the plan."
COUNT = "Count the files."
WAIT = "Wait for a new plan."
WORDS = {
  PLAN: """reviewer = chain("reviewer")
review = await prompt(str, "Review the plan.", on=reviewer)
ship = await prompt(str, "Ship it?", to=OPERATOR)
close(f"{review}, and the operator says {ship}")""",
  REVIEW: 'close("looks good")',
  COUNT: "files = 3",
  WAIT: "await wait(3600)",
}
DOLLARS = 0.3
NEW_PLAN = 4
asked: list[str] = []


def model(request: dict, write: Callable[..., None]) -> list[object]:
  said = json.dumps(request["messages"])
  task = next(one for one in WORDS if one in said)
  asked.append(task)
  write(text=WORDS[task])
  return ["assistant", WORDS[task], [100, 10, 0, 0, DOLLARS], None]


def console(answer: str | None) -> Generator[tuple | None, tuple]:
  while True:
    match (yield):
      case ("prompt", about, _, _, _, _, "operator"):
        yield ("started", about) if answer is None else ("done", about, answer)


def life(record: Path, answer: str | None) -> tuple[Live, str]:
  world = Live(str(record.parent), actor="claude-cli:opus", answer=model)
  held = world.opened(record, extensions=False)
  own = {} if furb.MONTY else {"kernel": kernel(vars(python)), "gate": sheet.gating(vars(python), furb_monty.gate)}
  return world, engine.boot(held, console=console(answer), **own, **world.ears)


async def until(holds: Callable[[], object]) -> None:
  while not holds():
    await asyncio.sleep(0.01)


def asking_operator(root: str) -> list[tuple]:
  return [one for one in engine.transcript(root) if one[0] == "prompt" and one[6] == OPERATOR]


async def first(record: Path) -> None:
  world, root = life(record, None)
  plan = engine.prompt(str, PLAN, on=root)
  await until(lambda: asking_operator(root))
  world.end()
  sys.stdout.write(f"{plan}\n")


async def second(record: Path, plan: str) -> None:
  world, root = life(record, "yes")
  try:
    engine.wake(root)
    assert await engine.Act(plan) == "looks good, and the operator says yes"
    assert asked == [], "the record answers each word of the first life"
    reviewer = next(one[1] for one in engine.transcript(root) if one[0] == "chain")
    assert engine.get(reviewer)[4] == "reviewer"

    budget = engine.chain("budget", on=root)
    engine.grant(usd=0.5, on=budget)
    engine.prompt(int, COUNT, on=budget)
    await until(lambda: engine.paused(budget))
    assert asked == [COUNT, COUNT], "the grant pauses the chain at its ceiling"

    door = engine.chain("door", on=root)
    waiting = engine.prompt(int, WAIT, on=door)
    await until(lambda: any(one[0] == "wait" for one in engine.transcript(door)))
    engine.write(Text(waiting, f"close({NEW_PLAN})"), on=door)
    assert await waiting == NEW_PLAN, "a write of the door of a prompt changes the plan of its ladder"
  finally:
    world.end()


def main() -> None:
  if sys.argv[1:]:
    asyncio.run(first(Path(sys.argv[1])))
    return
  record = Path(tempfile.mkdtemp(prefix="furb-example-")) / "life.jsonl"
  ran = subprocess.run([sys.executable, __file__, str(record)], stdout=subprocess.PIPE, text=True, check=True)
  plan = ran.stdout.split()[-1]
  sys.stdout.write(f"the first process ended while the operator was asked; {plan} waits in {record}\n")
  asyncio.run(second(record, plan))
  shutil.rmtree(record.parent)
  sys.stdout.write("the second process resumed it from the record, and every claim holds\n")


if __name__ == "__main__":
  main()
