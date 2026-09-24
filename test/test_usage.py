"""Usage, what one answer of a model cost."""

from conftest import STANDS, Sand, life, relived, said, settle
from furb import engine
from furb.engine import WORLD

COST = (80000, 30, 200, 10, 1.5)
"""The usage of one answer: the words read and written, those read again and kept, and the dollars."""


def test_what_one_answer_of_a_model_cost() -> None:
  """What one answer of a model cost: the words it read and wrote, of which the words it read again and the words it kept to read again, and its dollars."""
  words, wrote, again, kept, usd = COST
  assert (words, wrote) == (80000, 30)
  assert (again, kept) == (200, 10)
  assert usd == 1.5
  assert len(COST) == 5


async def test_a_usage_holds_the_token_counts_and_the_dollars_of_one_model_response() -> None:
  """A usage holds the token counts and the dollars of one model response."""
  sand = Sand(stands=STANDS, cost=COST)
  _, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert [usage for _, _, usage, _ in engine.turns(on=root)] == [None, COST, None]


async def test_the_share_of_the_window_it_filled_is_the_words_it_read_against_the_window_of_the_actor() -> None:
  """The share of the window it filled is the words it read against the window of the actor its rung names, in the standing the chain stood on when that rung was born, so no word of it says the share."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  asked = engine.prompt(int, "count", on=root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == asked]
  await engine.rung("actor = 'n/low'", on=root)
  engine.send("answer", step, ("assistant", "close(2)", COST, None), by=WORLD)
  assert await asked == 2
  (usage,) = [usage for role, _, usage, _ in engine.turns(on=root) if role == "assistant"]
  assert usage == COST and len(usage) == 5
  _, then = engine.ask("stand", root, step)
  assert isinstance(then, list) and engine.acts[step][6] == "m/low"
  assert usage[0] / (engine.offered(then, engine.acts[step][6]) or 0) == 0.2
  assert engine.modules[root]["actor"] == "n/low" and engine.offered(STANDS, "n/low") == 200000
  first = Sand(stands=STANDS)
  _, root = life(first)
  engine.prompt(int, "count", on=root)
  await settle()
  gone = [[STANDS[0][0], STANDS[0][2]], "/w", "n/low"]
  heard, _ = await relived(Sand(stands=gone), list(first.record))
  born = said(heard, "rung")[0]
  assert born[6] == "m/low" and [a[1] for a in said(heard, "stood")] == [root]
  assert engine.offered(gone, "m/low") is None
  _, then = engine.ask("stand", root, born[1])
  assert then == STANDS and engine.offered(then, born[6]) == 400000
