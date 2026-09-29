"""program, the words of the rungs that run on a chain."""

from conftest import BAD, born, fresh, said, settle, threaded, written
from furb import engine
from furb.engine import OPERATOR, Text


async def test_program_gives_the_program_of_a_chain() -> None:
  """program gives the program of a chain: the word of every rung that runs on it since its last module, as python, each under the name of the rung that the word first ran in, which is the rung itself, the rung that a rung which retells retells, or a told rung, in order, as the runs of its transcript say."""
  sand, log, root = born()
  mine = engine.rung("mine = 1", on=root)
  await mine
  sand.script[root] = ["<s:hi>\nhi\n</s:hi>\na = hi", "b = BAD", "close(len(a))"]
  one = engine.thread(int, "count", on=root)
  assert await one == 3
  await settle()
  first, refused, last = [a[1] for a in said(log, "rung") if a[2] == one]
  program = engine.program(root)
  findings = f"#{refused} refused\n{refused}_findings = {BAD!r}\n\n#{refused} closed\n{refused}_value = Refused()"
  assert list(program.items()) == [
    (f"{mine}_told", fresh(root, written(mine, "mine = 1"))),
    (mine, "mine = 1"),
    (f"{first}_told", f"{threaded(one, 'int', 'count')}\n\n#{first} advance on {one}"),
    (first, "hi = 'hi\\n'\n\n\na = hi"),
    (f"{refused}_told", f"#{refused} advance on {one}"),
    (f"{last}_told", f"{findings}\n\n#{last} advance on {one}"),
    (last, "close(len(a))"),
  ]
  assert refused not in program
  assert {a[4]: a[5] for a in engine.transcript(root) if a[0] == "run"} == program
  asked = engine.thread(int, "edit", to=OPERATOR, on=root)
  engine.write(Text(asked, "k = 1"), on=root)
  await settle()
  again = engine.program(root)
  (made,) = [a[1] for a in said(log, "rung") if a[2] == asked]
  held = f"#{one} closed\n{one}_value = 3\n\n{threaded(asked, 'int', 'edit')}"
  (retold, *_) = [a[1] for a in said(log, "rung") if a[2] == root]
  assert list(again) == [f"{retold}_told", *program, f"{made}_told", made]
  assert list(again.values()) == [held, *program.values(), written(made, "k = 1"), "k = 1"]
