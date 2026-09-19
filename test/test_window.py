"""WINDOW, the window of a model whose roster entry says none."""

import pytest

from conftest import Sand, attr, life, settle, tags
from furb import engine
from furb.engine import WINDOW

UNSAID = ((("operator", (), 200000), ("q", ("low",), 0)), "/w", "q/low")
"""A standing whose one model leaves its window unsaid."""


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_window_is_the_window_in_tokens_of_a_model_whose_roster_entry_does_not_say_one() -> None:
  """WINDOW is the window, in tokens, of a model whose roster entry does not say one."""
  assert WINDOW == 200000
  assert engine.offered(UNSAID, "q/low") == 0
  sand = Sand(stands=UNSAID, cost=(100000, 0, 0, 0, 0.0))
  _, root = life(sand)
  ceiling = engine.grant(share=0.9, on=root)
  await settle()
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert [attr(tag, "filled") for tag in tags(engine.turns(on=root), "ledger")] == [0.5]
  assert 100000 / WINDOW == 0.5
  engine.cancel(ceiling)
