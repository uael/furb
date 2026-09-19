"""gate, whether the word of a rung may run."""

import ast

import pytest

from conftest import STANDS, Py, Sand, attr, life, relived, settle, tags
from furb import engine
from furb.engine import Refused

CHECKS = (
  "def checks(id):\n"
  "  while True:\n"
  "    match (yield):\n"
  "      case ('read', qid, _, _, path) if path.startswith('check://'):\n"
  "        yield 'done', qid, Text(path, '\\n'.join(gate('BA' 'D', 'int')))\n"
  "\n"
  "act('check', '', checks)\n"
  "close(1)\n"
)
"""A word that opens an act which reads a word of its own against the gate when a door of that act is read.

The word it gates is spelled in two parts, so that the gate of the suite accepts this word itself. A word says a
gate only from an act of its own, since the Kernel is busy with the word while the word runs.
"""


class Strict(Py):
  """A Kernel that reads the value a word closes with against the name of the shape the word must give."""

  def gate(self, rung: str, ladder: list[str], shape: str) -> list[str]:
    """What it finds against a word: what python finds, and a close of a plain value of another shape."""
    found = super().gate(rung, ladder, shape)
    calls = [node for node in ast.walk(ast.parse(rung)) if isinstance(node, ast.Call) and node.args]
    gave = [node.args[0] for node in calls if getattr(node.func, "id", "") == "close"]
    wrong = [one for one in gave if isinstance(one, ast.Constant) and shape and type(one.value).__name__ != shape]
    return found + [f"a close of {type(one.value).__name__} is no {shape}" for one in wrong]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_whether_the_word_of_a_rung_may_run() -> None:
  """Whether the word of a rung may run: the Kernel reads it against the rungs of its chain before it and the name of the shape the word must give, and it finds nothing when the word may run."""
  sand, py = Sand(stands=STANDS), Py()
  _, root = life(sand, kernel=py)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  assert py.gates == [("k = 1", [], ""), ("close(k + 1)", ["k = 1"], "int")]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_word_of_a_rung_runs_only_if_the_gate_accepts_the_word() -> None:
  """The word of a rung runs only if the gate accepts the word."""
  sand, py = Sand(stands=STANDS), Py()
  _, root = life(sand, kernel=py)
  with pytest.raises(Refused):
    await engine.rung("BAD = 1", on=root)
  assert py.ran == [] and "BAD" not in engine.modules[root]
  assert await engine.rung("k = 1", on=root) is None
  assert py.ran == ["k = 1"] and engine.modules[root]["k"] == 1


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_gate_checks_the_word_of_a_rung_against_the_rungs_before_it_in_record_order() -> None:
  """The gate checks the word of a rung against the rungs before it in record order."""
  sand, py = Sand(stands=STANDS), Py()
  _, root = life(sand, kernel=py)
  await engine.rung("a = 1", on=root)
  await engine.rung("b = 2", on=root)
  await engine.rung("c = 3", on=root)
  assert [ladder for _, ladder, _ in py.gates] == [[], ["a = 1"], ["a = 1", "b = 2"]]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_response_that_is_not_python_is_a_finding_like_any_other() -> None:
  """A response that is not python is a finding like any other."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["this is no python at all", "close(1)"]
  assert await engine.prompt(int, "try", on=root) == 1
  await settle()
  bad = tags(engine.turns(on=root), "refused")
  assert len(bad) == 1 and isinstance(bad[0][2], str) and bad[0][2].startswith("not python: ")


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_gate_gives_no_finding_when_the_gate_accepts_the_rung() -> None:
  """The gate gives no finding when the gate accepts the rung."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert engine.gate("k = 1", on=root) == []
  assert engine.gate("close(1)", "int", on=root) == []


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_word_of_a_rung_is_gated_again_in_every_life_that_runs_it() -> None:
  """The word of a rung is gated again in every life that runs it, since the gate is of the moment and its findings are kept by nobody."""
  sand, py = Sand(stands=STANDS), Py()
  _, root = life(sand, kernel=py)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert py.gated == ["close(1)"]
  assert [fact for _, fact, *_ in sand.record if fact[0] == "gate"] == []
  again = Py()
  _, over = await relived(Sand(stands=STANDS), list(sand.record), again)
  assert over == root and again.gated == ["close(1)"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_gate_refuses_a_return_whose_value_does_not_have_the_shape() -> None:
  """The gate refuses a word whose close carries a value that does not have the shape."""
  sand, py = Sand(stands=STANDS), Strict()
  _, root = life(sand, kernel=py)
  sand.script[root] = ["close('nope')", "close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  assert [tag[2] for tag in tags(engine.turns(on=root), "refused")] == ["a close of str is no int"]
  assert py.ran == ["close(1)"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_gate_tells_the_shape_it_read_against_and_the_findings_as_its_body() -> None:
  """gate tells the shape it read against and the findings as its body."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = [CHECKS]
  assert await engine.prompt(int, "open a door", on=root) == 1
  await settle()
  found = "BAD in rung, against int after 1 rungs"
  assert engine.read("check://one", on=root).content == found
  told = tags(engine.turns(on=root), "gate")
  assert [(attr(tag, "returns"), tag[2]) for tag in told] == [("int", found)]
