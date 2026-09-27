"""WINDOW, the window of a model whose roster entry says none."""

from conftest import born, heads, said, settle
from furb import engine
from furb.engine import WINDOW

UNSAID: list = [[["operator", [], 200000], ["q", ["low"], 0]], "/w", "q/low"]
"""A standing whose one model leaves its window unsaid."""


async def test_window_is_the_window_in_tokens_of_a_model_whose_roster_entry_does_not_say_one() -> None:
  """WINDOW is the window, in tokens, of a model whose roster entry does not say one."""
  assert WINDOW == 200000
  assert engine.offered(UNSAID[0], "q/low") == 0
  sand, log, root = born(stands=UNSAID, cost=(100000, 0, 0, 0, 0.0))
  ceiling = engine.grant(share=0.9, on=root)
  await settle()
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  step = said(log, "reply")[0][2]
  assert [one for one in heads(engine.turns(on=root)) if " ledger " in one] == [f"#{step} ledger spent=0.0 filled=0.5"]
  assert 100000 / WINDOW == 0.5
  engine.cancel(ceiling)
