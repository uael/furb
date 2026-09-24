"""Drift, what a life fails with when an act of it is not the one the record holds."""

import pytest

from conftest import STANDS, Sand, life, plain, relived, settle
from furb import engine
from furb.engine import Drift


async def test_what_a_life_fails_with_when_an_act_of_it_is_not_the_one_the_record_holds() -> None:
  """What a life fails with when an act of it is not the one the record holds, which the journal raises, so that it comes out of the entry the operator went in by and the life goes on with nothing."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["import random\nx = bash(f'echo {random.random()}')\nclose((await x).code)"]
  assert await engine.prompt(int, "roll", on=root) == 0
  await settle()
  later = Sand(stands=STANDS)
  with pytest.raises(Drift, match=r"^bash1 drifts$"):
    life(later, list(sand.record))
  assert later.record == []
  torn = Sand(stands=STANDS, auto=False)
  _, root = life(torn)
  step = engine.rung("x = bash('sleep 9')", on=root)
  await settle()
  with pytest.raises(Drift, match=r"^bash1 drifts$"):
    life(Sand(stands=STANDS), [e for e in torn.record if e[0][1] != step])


async def test_an_act_whose_words_are_not_the_ones_the_record_holds_is_a_drift() -> None:
  """An act whose words are not the ones the record holds under its name is a drift, which it raises; a drift is never asked of a query, since a query is answered again and not made again."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = [
    "import random\nread(f'{random.random()}.txt')\nx = bash('echo one')\nn = (await x).code",
    "close(chain('side', source=__name__, filter=take(x, inside=False)))",
  ]
  side = await engine.prompt(str, "fork from a word", on=root)
  await settle(200)
  assert engine.modules[side]["n"] == 0
  kept = plain(sand.record)
  assert len(kept) == len(sand.record)
  _, over = await relived(Sand(stands=STANDS), kept)
  assert over == root and engine.modules[side]["n"] == 0


async def test_a_drift_breaks_the_journal_which_keeps_nothing_more() -> None:
  """A drift breaks the journal, which keeps nothing more, and the life runs on with nothing kept."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["import random\nx = bash(f'echo {random.random()}')\nclose((await x).code)"]
  assert await engine.prompt(int, "roll", on=root) == 0
  later = Sand(stands=STANDS)
  with pytest.raises(Drift):
    life(later, list(sand.record))
  assert later.record == []
  two = engine.chain("two")
  await settle()
  assert two == "chain2" and later.record == []
