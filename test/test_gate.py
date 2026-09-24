"""gate, whether the word of a rung may run."""

import pytest

from conftest import BAD, STANDS, Sand, bindings, findings, gated, gatings, life, paragraphs, ran, relived, said, settle
from furb import engine
from furb.engine import Refused


async def test_whether_the_word_of_a_rung_may_run() -> None:
  """Whether the word of a rung may run: the gate reads it after the program of its chain, and it finds nothing when the word may run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  await settle()
  assert gatings() == [("k = 1", []), ("close(k + 1)", ["k = 1", bindings(root, act, "int")])]
  assert findings(log) == [[], []]


async def test_the_word_of_a_rung_runs_only_if_the_gate_accepts_the_word_or_if_the_chain_wrote_it() -> None:
  """The word of a rung runs only if the gate accepts the word, or if the chain wrote it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  with pytest.raises(Refused):
    await engine.rung("k = BAD", on=root)
  assert findings(log) == [[BAD]]
  assert ran(log) == [] and "k" not in engine.modules[root]
  assert await engine.rung("k = 1", on=root) is None
  assert ran(log) == ["k = 1"] and engine.modules[root]["k"] == 1
  sand.script[root] = ["close(k)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  wrote = bindings(root, act, "int")
  assert [a[4] for a in said(log, "rung") if a[2] == root] == [wrote]
  assert [word for word, _ in gatings()] == ["k = BAD", "k = 1", "close(k)"]
  assert ran(log) == ["k = 1", wrote, "close(k)"] and engine.modules[root][act] == act


async def test_the_gate_checks_the_word_of_a_rung_against_the_rungs_before_it_in_record_order() -> None:
  """The gate checks the word of a rung against the rungs before it in record order."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("a = 1", on=root)
  await engine.rung("b = a + 1", on=root)
  with pytest.raises(Refused):
    await engine.rung("c = later", on=root)
  await engine.rung("later = 3", on=root)
  await engine.rung("c = later", on=root)
  assert gatings() == [
    ("a = 1", []),
    ("b = a + 1", ["a = 1"]),
    ("c = later", ["a = 1", "b = a + 1"]),
    ("later = 3", ["a = 1", "b = a + 1"]),
    ("c = later", ["a = 1", "b = a + 1", "later = 3"]),
  ]
  assert [bool(found) for found in findings(log)] == [False, False, True, False, False]


async def test_a_response_that_is_not_python_is_a_finding_like_any_other() -> None:
  """A response that is not python is a finding like any other."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["this is no python at all", "close(1)"]
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  await settle()
  assert gated(log) == ["this is no python at all", "close(1)"]
  (found,), none = findings(log)
  assert found.startswith("line 1: ") and none == []
  step = said(log, "rung")[0][1]
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{step} ")] == [
    f"#{step} advance on {act}",
    f"#{step} refused\n# {found}",
    f"#{step} closed Refused()",
  ]
  assert ran(log) == [bindings(root, act, "int"), "close(1)"]


async def test_the_gate_gives_no_finding_when_the_gate_accepts_the_rung() -> None:
  """The gate gives no finding when the gate accepts the rung."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.gate("k = 1", on=root) == []
  assert engine.gate("close(1)", on=root) == []
  assert engine.gate("k = BAD", on=root) == [BAD]


async def test_the_word_of_a_rung_is_gated_again_in_every_life_that_runs_it() -> None:
  """The word of a rung is gated again in every life that runs it, since the gate is of the moment and its findings are kept by nobody."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert gated(log) == ["close(1)"] and findings(log) == [[]]
  assert [fact for fact, *_ in sand.record if fact[0] == "gate"] == []
  again, over = await relived(Sand(stands=STANDS), list(sand.record))
  assert over == root and gated(again) == ["close(1)"] and findings(again) == [[]]
