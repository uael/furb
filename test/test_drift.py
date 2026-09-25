"""Drift, what a life fails with when an act of it is not the one the record holds."""

import pytest

from conftest import STANDS, Sand, life, plain, relived, settle, sown
from furb import engine
from furb.engine import Drift


async def test_what_a_life_fails_with_when_an_act_of_it_is_not_the_one_the_record_holds() -> None:
  """What a life fails with when an act of it is not the one the record holds, which the record raises, so that it comes out of the entry the operator went in by and the life goes on with nothing."""
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
  """An act whose words are not the ones the record holds under its name is a drift, which it raises."""
  sand = sown()
  _, root = life(sand)
  assert await engine.rung("t = read('a.txt')", on=root) is None
  kept = [((*e[0][:4], "b.txt"),) if e[0][:2] == ("read", "read1") else e for e in plain(sand.record)]
  with pytest.raises(Drift, match=r"^read1 drifts$"):
    life(Sand(stands=STANDS), kept)
  _, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert over == root and engine.module(over)["t"].content == "one\ntwo\n"


async def test_a_drift_breaks_the_record_which_keeps_nothing_more() -> None:
  """A drift breaks the record, which keeps nothing more, and the life runs on with nothing kept."""
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
