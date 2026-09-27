"""Usage, what one answer of a model cost."""

from conftest import STANDS, Sand, born, heads, relived, said, settle, world_says
from furb import engine

USAGE = (80000, 30, 200, 10, 1.5)
"""The usage of one answer: the words read and written, those read again and kept, and the dollars."""


def test_what_one_answer_of_a_model_cost() -> None:
  """What one answer of a model cost: the words it read and wrote, of which the words it read again and the words it kept to read again, and its dollars."""
  words, wrote, again, kept, usd = USAGE
  assert (words, wrote) == (80000, 30)
  assert (again, kept) == (200, 10)
  assert usd == 1.5
  assert len(USAGE) == 5


async def test_a_usage_holds_the_token_counts_and_the_dollars_of_one_model_response() -> None:
  """A usage holds the token counts and the dollars of one model response."""
  _, _, root = born("close(1)", cost=USAGE)
  assert await engine.prompt(int, "count", on=root) == 1
  assert [usage for _, _, usage, _ in engine.turns(on=root)] == [None, USAGE, None]


async def test_the_share_of_the_window_it_filled_is_the_words_it_read_against_the_window_of_the_actor() -> None:
  """The share of the window it filled is the words it read against the window of the actor its rung names, in the standing where the answer lands, so no word of it says the share."""
  sand, log, root = born(cost=USAGE)
  ceiling = engine.grant(usd=10.0, on=root)
  await settle()
  sand.script[root] = ["a = 1", "close(2)"]
  assert await engine.prompt(int, "count", on=root) == 2
  one, two = [a[2] for a in said(log, "reply")]
  assert [line for line in heads(engine.turns(on=root)) if " ledger " in line] == [
    f"#{one} ledger spent=1.5 filled=0.2",
    f"#{two} ledger spent=3.0 filled=0.2",
  ]
  assert engine.offered(STANDS[0], "m/low") == 400000 and USAGE[0] / 400000 == 0.2
  engine.cancel(ceiling)
  _, _, root = born("close(2)", cost=USAGE)
  asked = engine.prompt(int, "count", on=root)
  engine.rung("actor = 'n/low'", on=root)
  later = engine.grant(usd=10.0, on=root)
  assert await asked == 2
  assert [line.split("filled=")[1] for line in heads(engine.turns(on=root)) if " ledger " in line] == ["0.2"]
  engine.cancel(later)
  first, _, root = born(cost=USAGE)
  engine.prompt(int, "count", on=root)
  await settle()
  wider: list = [[STANDS[0][0], ["m", ["low"], 800000]], "/w", "m/low"]
  heard, _ = await relived(Sand(stands=wider, cost=USAGE), list(first.record))
  step = said(heard, "rung")[0]
  assert step[6] == "m/low" and [a[1] for a in said(heard, "stand")] == ["stand1", "stand2"]
  assert engine.offered(wider[0], "m/low") == 800000
  engine.grant(share=0.9, on=root)
  (asked,) = [a for a in engine.transcript(root) if a[0] == "reply"]
  world_says("done", asked[1], ("assistant", "close(3)", USAGE, None))
  await settle()
  assert [line.split("filled=")[1] for line in heads(engine.turns(on=root)) if " ledger " in line] == ["0.1"]
