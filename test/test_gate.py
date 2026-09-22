"""gate, whether the word of a rung may run."""

import pytest

from conftest import STANDS, Sand, findings, gated, life, ran, refusals, relived, settle, tags
from furb import engine
from furb.engine import Refused


async def test_whether_the_word_of_a_rung_may_run() -> None:
  """Whether the word of a rung may run: the gate reads it after the program of its chain, and it finds nothing when the word may run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  assert gated(log) == ["k = 1", "close(k + 1)"] and findings(log) == [[], []]


async def test_the_word_of_a_rung_runs_only_if_the_gate_accepts_the_word() -> None:
  """The word of a rung runs only if the gate accepts the word."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  with pytest.raises(Refused):
    await engine.rung("k = BAD", on=root)
  assert ran(log) == [] and "k" not in engine.modules[root]
  assert await engine.rung("k = 1", on=root) is None
  assert ran(log) == ["k = 1"] and engine.modules[root]["k"] == 1


async def test_the_gate_checks_the_word_of_a_rung_against_the_rungs_before_it_in_record_order() -> None:
  """The gate checks the word of a rung against the rungs before it in record order."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("a = 1", on=root)
  with pytest.raises(Refused):
    await engine.rung("b = BAD", on=root)
  await engine.rung("c = 3", on=root)
  with pytest.raises(Refused):
    await engine.rung("d = BAD", on=root)
  # The gate of the harness counts the rungs it read the word against, and a refused word joins none of them.
  assert gated(log) == ["a = 1", "b = BAD", "c = 3", "d = BAD"]
  assert findings(log) == [[], ["BAD is no name, after 1 rungs"], [], ["BAD is no name, after 2 rungs"]]


async def test_a_response_that_is_not_python_is_a_finding_like_any_other() -> None:
  """A response that is not python is a finding like any other."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["this is no python at all", "close(1)"]
  assert await engine.prompt(int, "try", on=root) == 1
  await settle()
  bad = tags(engine.turns(on=root), "refused")
  assert len(bad) == 1 and [tag[2] for tag in bad] == refusals(log)


async def test_the_gate_gives_no_finding_when_the_gate_accepts_the_rung() -> None:
  """The gate gives no finding when the gate accepts the rung."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.gate("k = 1", on=root) == []
  assert engine.gate("close(1)", on=root) == []


async def test_the_word_of_a_rung_is_gated_again_in_every_life_that_runs_it() -> None:
  """The word of a rung is gated again in every life that runs it, since the gate is of the moment and its findings are kept by nobody."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert gated(log) == ["close(1)"]
  assert [fact for _, fact, *_ in sand.record if fact[0] == "gate"] == []
  again, over = await relived(Sand(stands=STANDS), list(sand.record))
  assert over == root and gated(again) == ["close(1)"]


async def test_gate_tells_the_findings_as_its_body() -> None:
  """gate tells the findings as its body."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["found = gate('BAD')\nclose(len(found))"]
  assert await engine.prompt(int, "ask the gate", on=root) == 1
  await settle()
  found = "BAD is no name, after 1 rungs"
  assert engine.modules[root]["found"] == [found]
  assert [tag[2] for tag in tags(engine.turns(on=root), "gate")] == [found]
