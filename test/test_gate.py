"""gate, whether the word of a rung may run."""

import pytest

from conftest import (
  BAD,
  Sand,
  born,
  findings,
  fresh,
  gated,
  gatings,
  paragraphs,
  ran,
  relived,
  said,
  settle,
  threaded,
  written,
)
from furb import engine
from furb.engine import Refused


async def test_whether_the_word_of_a_rung_may_run() -> None:
  """Whether the word of a rung may run: the gate reads it after the program of its chain, and it finds nothing when the word may run."""
  sand, log, root = born()
  laid = engine.rung("k = 1", on=root)
  await laid
  sand.script[root] = ["close(k + 1)"]
  act = engine.thread(int, "count", on=root)
  assert await act == 2
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  told = [fresh(root, written(laid, "k = 1")), f"{threaded(act, 'int', 'count')}\n\n#{step} advance on {act}"]
  assert gatings(log) == [("k = 1", told[:1]), ("close(k + 1)", [told[0], "k = 1", told[1]])]
  assert findings(log) == [[], []]


async def test_the_word_of_a_rung_runs_only_if_the_gate_accepts_the_word_or_if_the_chain_wrote_it() -> None:
  """The word of a rung runs only if the gate accepts the word, or if the chain wrote it."""
  sand, log, root = born()
  bad = engine.rung("k = BAD", on=root)
  with pytest.raises(Refused):
    await bad
  assert findings(log) == [[BAD]]
  told = [fresh(root, written(bad, "k = BAD"))]
  assert ran(log) == told and "k" not in engine.module(root)
  laid = engine.rung("k = 1", on=root)
  assert await laid is None
  refused = f"#{bad} refused\n{bad}_findings = {BAD!r}\n\n#{bad} closed\n{bad}_value = Refused()"
  told.append(f"{refused}\n\n{written(laid, 'k = 1')}")
  assert ran(log) == [*told, "k = 1"] and engine.module(root)["k"] == 1
  sand.script[root] = ["close(k)"]
  act = engine.thread(int, "count", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  told.append(f"{threaded(act, 'int', 'count')}\n\n#{step} advance on {act}")
  assert [a[5] for a in said(log, "run") if a[4].endswith("_told")] == told
  assert [word for word, _ in gatings(log)] == ["k = BAD", "k = 1", "close(k)"]
  assert ran(log) == [*told[:2], "k = 1", told[2], "close(k)"] and engine.module(root)[act] == act


async def test_the_gate_checks_the_word_of_a_rung_against_the_rungs_before_it_in_record_order() -> None:
  """The gate checks the word of a rung against the rungs before it in record order."""
  _, log, root = born()
  first = engine.rung("a = 1", on=root)
  await first
  second = engine.rung("b = a + 1", on=root)
  await second
  early = engine.rung("c = later", on=root)
  with pytest.raises(Refused):
    await early
  later = engine.rung("later = 3", on=root)
  await later
  again = engine.rung("c = later", on=root)
  await again
  (finding,) = next(one for one in findings(log) if one)
  refused = f"#{early} refused\n{early}_findings = {finding!r}\n\n#{early} closed\n{early}_value = Refused()"
  told = [
    fresh(root, written(first, "a = 1")),
    written(second, "b = a + 1"),
    written(early, "c = later"),
    f"{refused}\n\n{written(later, 'later = 3')}",
    written(again, "c = later"),
  ]
  assert gatings(log) == [
    ("a = 1", told[:1]),
    ("b = a + 1", [told[0], "a = 1", told[1]]),
    ("c = later", [told[0], "a = 1", told[1], "b = a + 1", told[2]]),
    ("later = 3", [told[0], "a = 1", told[1], "b = a + 1", told[2], told[3]]),
    ("c = later", [told[0], "a = 1", told[1], "b = a + 1", told[2], told[3], "later = 3", told[4]]),
  ]
  assert [bool(found) for found in findings(log)] == [False, False, True, False, False]


async def test_a_response_that_is_not_python_is_a_finding_like_any_other() -> None:
  """A response that is not python is a finding like any other."""
  _, log, root = born("this is no python at all", "close(1)")
  act = engine.thread(int, "try", on=root)
  assert await act == 1
  await settle()
  assert gated(log) == ["this is no python at all", "close(1)"]
  (found,), none = findings(log)
  assert found.startswith("line 1: ") and none == []
  step = said(log, "rung")[0][1]
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{step} ")] == [
    f"#{step} advance on {act}",
    f"#{step} refused\n{step}_findings = {found!r}",
    f"#{step} closed\n{step}_value = Refused()",
  ]
  (_, last) = [a[1] for a in said(log, "rung") if a[2] == act]
  told = [
    fresh(root, threaded(act, "int", "try"), f"#{step} advance on {act}"),
    f"#{step} refused\n{step}_findings = {found!r}\n\n#{step} closed\n{step}_value = Refused()\n\n#{last} advance on {act}",
  ]
  assert ran(log) == [told[0], told[1], "close(1)"]


async def test_the_gate_gives_no_finding_when_the_gate_accepts_the_rung() -> None:
  """The gate gives no finding when the gate accepts the rung."""
  _, _, root = born()
  assert engine.gate("k = 1", on=root) == []
  assert engine.gate("close(1)", on=root) == []
  assert engine.gate("k = BAD", on=root) == [BAD]


async def test_the_journal_keeps_what_the_gate_found() -> None:
  """The journal keeps what the gate found, since the gate is of the outside, so a later life reads the same findings and asks the gate nothing again."""
  sand, log, root = born("close(1)")
  assert await engine.thread(int, "count", on=root) == 1
  await settle()
  assert gated(log) == ["close(1)"] and findings(log) == [[]]
  assert [fact[1] for fact, *_ in sand.record if fact[0] == "gate"] == ["gate1"]
  again, over = await relived(Sand(), list(sand.record))
  assert over == root and gated(again) == ["close(1)"] and findings(again) == [[]]
  assert [a[2] for a in said(again, "done") if a[1] == "gate1"] == ["journal"]
