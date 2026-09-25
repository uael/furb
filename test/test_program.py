"""program, the words of the rungs that run on a chain."""

from conftest import bindings, life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Text


async def test_program_gives_the_program_of_a_chain() -> None:
  """program gives the program of a chain: the word of every rung that runs on it since its last module, as python, each under the name of that rung, in order, as the runs of its transcript say."""
  sand = sown()
  log, root = life(sand)
  mine = engine.rung("mine = 1", on=root)
  await mine
  sand.script[root] = ["<S1>\nhi\n</S1>\na = S1", "b = BAD", "close(len(a))"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 3
  await settle()
  first, refused, last = [a[1] for a in said(log, "rung") if a[2] == one]
  (bind,) = [a[1] for a in said(log, "rung") if a[2] == root]
  program = engine.program(root)
  assert list(program.items()) == [
    (mine, "mine = 1"),
    (bind, bindings(root, one, "int")),
    (first, "S1 = 'hi\\n'\n\n\na = S1"),
    (last, "close(len(a))"),
  ]
  assert refused not in program
  assert {a[4]: a[5] for a in engine.transcript(root) if a[0] == "run"} == program
  asked = engine.prompt(int, "edit", to=OPERATOR, on=root)
  engine.write(Text(asked, "k = 1"), on=root)
  await settle()
  again = engine.program(root)
  assert list(again.values()) == [*program.values(), "k = 1"] and not set(again) & set(program)
