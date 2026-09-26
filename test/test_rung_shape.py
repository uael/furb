"""Rung, the word, the rung it retells and the actor of the run of one word."""

from conftest import born, said, settle
from furb import engine


async def test_a_rung_carries_the_word_the_rung_it_retells_and_the_actor() -> None:
  """A rung carries the word, the rung it retells and the actor."""
  sand, log, root = born()
  laid = engine.rung("k = 21", on=root)
  assert await laid is None
  sand.script[root] = ["close(k + 1)"]
  act = engine.prompt(int, "count", to="m/high", on=root)
  assert await act == 22
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (binding,) = [a[1] for a in said(log, "rung") if a[2] == root]
  wrote = f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"
  assert [a[4:] for a in said(log, "rung")] == [("k = 21", "", ""), ("", "", "m/high"), (wrote, "", "")]
  assert [a[1] for a in said(log, "rung")] == [laid, step, binding]
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert [a[4:] for a in said(log, "rung") if a[3] == twin] == [
    ("k = 21", laid, ""),
    (wrote, binding, ""),
    ("close(k + 1)", step, ""),
  ]
