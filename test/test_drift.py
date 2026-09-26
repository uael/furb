"""Drift, what a life fails with when an act of it is not the one the record holds."""

import pytest

from conftest import WORLD, Sand, life, plain, relived, settle, sown
from furb import engine
from furb.engine import Drift


async def test_what_a_life_fails_with_when_an_act_of_it_is_not_the_one_the_record_holds() -> None:
  """What a life fails with when an act of it is not the one the record holds, which the journal raises, so that it comes out of the entry the operator went in by and the life goes on with nothing."""
  sand = Sand()
  _, root = life(sand)
  sand.script[root] = ["import random\nx = bash(f'echo {random.random()}')\nclose((await x).code)"]
  assert await engine.prompt(int, "roll", on=root) == 0
  await settle()
  later = Sand()
  with pytest.raises(Drift, match=r"^bash1 drifts$"):
    life(later, list(sand.record))
  assert later.record == []
  torn = Sand(auto=False)
  _, root = life(torn)
  step = engine.rung("x = bash('sleep 9')", on=root)
  await settle()
  with pytest.raises(Drift, match=r"^bash1 drifts$"):
    life(Sand(), [e for e in torn.record if e[0][1] != step])


async def test_an_act_whose_words_are_not_the_ones_the_record_holds_is_a_drift() -> None:
  """An act whose words are not the ones the record holds under its name is a drift, which it raises."""
  sand = sown()
  _, root = life(sand)
  assert await engine.rung("t = read('a.txt')", on=root) is None
  kept = [((*e[0][:4], "b.txt"),) if e[0][:2] == ("read", "read1") else e for e in plain(sand.record)]
  with pytest.raises(Drift, match=r"^read1 drifts$"):
    life(Sand(), kept)
  _, over = await relived(Sand(), plain(sand.record))
  assert over == root and engine.module(over)["t"].content == "one\ntwo\n"


async def test_a_drift_breaks_the_journal_which_keeps_nothing_more() -> None:
  """A drift breaks the journal, which keeps nothing more, and the life runs on with nothing kept."""
  sand = Sand(auto=False)
  _, root = life(sand)
  sand.script[root] = ["import random\nx = bash(f'echo {random.random()}')\nclose(None)"]
  await engine.prompt(None, "roll", on=root)
  kept = engine.bash("echo kept", on=root)
  await settle()
  assert kept == "bash2" and ("started", "bash2", WORLD) in [e[0] for e in sand.record]
  later = Sand()
  with pytest.raises(Drift):
    life(later, list(sand.record))
  assert later.record == []
  two = engine.chain("two")
  await settle()
  assert two == "chain2" and later.record == []
  assert engine.clock(on=two) == 1001.0
  fresh = engine.bash("echo new", on=two)
  await settle()
  assert fresh == "bash2" and [a[4] for a in later.calls if a[0] == "bash"] == ["echo new"]
  assert (await fresh).code == 0 and later.record == []
