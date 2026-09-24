"""WINDOW, the window of a model whose roster entry says none."""

from conftest import Sand, heads, life, said, settle
from furb import engine
from furb.engine import WINDOW

UNSAID = [[["operator", [], 200000], ["q", ["low"], 0]], "/w", "q/low"]
"""A standing whose one model leaves its window unsaid."""


async def test_window_is_the_window_in_tokens_of_a_model_whose_roster_entry_does_not_say_one() -> None:
  """WINDOW is the window, in tokens, of a model whose roster entry does not say one."""
  assert WINDOW == 200000
  assert engine.offered(UNSAID, "q/low") == 0
  sand = Sand(stands=UNSAID, cost=(100000, 0, 0, 0, 0.0))
  log, root = life(sand)
  ceiling = engine.grant(share=0.9, on=root)
  await settle()
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  step = said(log, "answer")[0][1]
  assert [one for one in heads(engine.turns(on=root)) if " ledger " in one] == [f"#{step} ledger spent=0.0 filled=0.5"]
  assert 100000 / WINDOW == 0.5
  engine.cancel(ceiling)


async def test_the_window_that_a_roster_entry_leaves_unsaid_is_the_window_that_the_file_names() -> None:
  """The window that a roster entry leaves unsaid is the window that the file names."""
  assert engine.WINDOW == 200000
  ledgers = []
  for roster in ([["plain", [], 0]], [["plain", [], 400000]]):
    sand = Sand(stands=[roster, "/w", "plain"], cost=(100000, 0, 0, 0, 0.0))
    _, root = life(sand)
    sand.script[root] = ["close(1)"]
    engine.grant(share=0.9, on=root)
    assert await engine.prompt(int, "count", on=root) == 1
    await settle()
    ledgers += [one for one in heads(engine.turns(on=root)) if one.startswith("#rung1 ledger ")]
  assert ledgers == [
    f"#rung1 ledger spent=0.0 filled={100000 / engine.WINDOW}",
    f"#rung1 ledger spent=0.0 filled={100000 / 400000}",
  ]
