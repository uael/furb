"""Rung, the word, the rung it retells and the actor of the run of one word."""

from conftest import born, chained, fresh, prompted, said, settle, written
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
  assert [a[4:] for a in said(log, "rung")] == [("k = 21", "", ""), ("", "", "m/high")]
  assert [a[1] for a in said(log, "rung")] == [laid, step]
  twin = await chained("twin", root, 300)
  told = [fresh(root, written(laid, "k = 21")), f"{prompted(act, 'int', 'count')}\n\n#{step} advance on {act}"]
  assert [a[4:] for a in said(log, "rung") if a[3] == twin] == [
    (told[0], f"{laid}_told", ""),
    ("k = 21", laid, ""),
    (told[1], f"{step}_told", ""),
    ("close(k + 1)", step, ""),
  ]
