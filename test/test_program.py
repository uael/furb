"""program, the words of the rungs that run on a chain."""

from conftest import born, faulted, fresh, prompted, said, settle
from furb import engine
from furb.engine import OPERATOR, Text


async def test_program_gives_the_program_of_a_chain() -> None:
  """program gives the program of a chain: the word of every rung that runs on it since its last module, as python, each under the name of the rung that the word first ran in, which is the rung itself, the rung that a rung which retells retells, or a told rung, in order, as the runs of its transcript say."""
  sand, log, root = born()
  mine = engine.rung("mine = 1", on=root)
  await mine
  sand.script[root] = ["<s:hi>\nhi\n</s:hi>\na = hi", "b = BAD", "close(len(a))"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 3
  await settle()
  first, refused, last = [a[1] for a in said(log, "rung") if a[2] == one]
  program = engine.program(root)
  findings = f"#{refused} refused\n{refused}_findings = {faulted('b = BAD')!r}"
  assert list(program.items()) == [
    (f"{mine}_told", fresh(root)),
    (mine, "mine = 1"),
    (f"{first}_told", f"{prompted(one, 'int', 'count')}\n\n#{first} advance on {one}"),
    (first, "hi = 'hi\\n'\n\n\na = hi"),
    (f"{refused}_told", f"#{first} told nothing\n\n#{refused} advance on {one}"),
    (f"{last}_told", f"{findings}\n\n#{last} advance on {one}"),
    (last, "close(len(a))"),
  ]
  assert refused not in program
  assert {a[4]: a[5] for a in engine.transcript(root) if a[0] == "run"} == program
  asked = engine.prompt(int, "edit", to=OPERATOR, on=root)
  engine.write(Text(asked, "k = 1"), on=root)
  await settle()
  (made,) = [a[1] for a in said(log, "rung") if a[2] == asked]
  held = prompted(asked, "int", "edit")
  assert list(engine.program(root).items()) == [*program.items(), (f"{made}_told", held), (made, "k = 1")]
  engine.write(Text(asked, ""), on=root)
  await settle()
  assert list(engine.program(root).items()) == [*program.items(), (f"{made}_told", held)]
  assert len(said(engine.transcript(root), "module")) == 2 and "k" not in engine.module(root)
