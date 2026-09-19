"""Usage, what one answer of a model cost."""

import pytest

from conftest import STANDS, Sand, attr, life, settle, tags
from furb import engine

COST = (80000, 30, 200, 10, 1.5)
"""The usage of one answer: the words read and written, those read again and kept, and the dollars."""


def test_what_one_answer_of_a_model_cost() -> None:
  """What one answer of a model cost: the words it read and wrote, of which the words it read again and the words it kept to read again, and its dollars."""
  words, wrote, again, kept, usd = COST
  assert (words, wrote) == (80000, 30)
  assert (again, kept) == (200, 10)
  assert usd == 1.5
  assert len(COST) == 5


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_usage_holds_the_token_counts_and_the_dollars_of_one_model_response() -> None:
  """A usage holds the token counts and the dollars of one model response."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert [usage for _, _, usage, _ in engine.turns(on=root)] == [None, COST, None]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_share_of_the_window_it_filled_is_the_words_it_read_against_the_window_of_the_actor() -> None:
  """The share of the window it filled is the words it read against the window of the actor, so no word of it says the share."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = life(sand)
  ceiling = engine.grant(usd=10.0, on=root)
  await settle()
  sand.script[root] = ["a = 1", "close(2)"]
  assert await engine.prompt(int, "count", on=root) == 2
  assert [attr(tag, "filled") for tag in tags(engine.turns(on=root), "ledger")] == [0.2, 0.2]
  assert engine.offered(STANDS, "m/low") == 400000 and COST[0] / 400000 == 0.2
  assert len(COST) == 5
  engine.cancel(ceiling)
