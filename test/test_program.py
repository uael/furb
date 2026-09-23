"""Program, the question of the words of the rungs of a chain that run, as python."""

import pytest

from conftest import Sand, life, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_a_program_is_the_question_of_the_words_of_the_rungs_of_a_chain_that_run_as_python() -> None:
  """A program is the question of the words of the rungs of a chain that run, as python, each with the name of its rung, which the chain answers."""
  sand: Sand = sown()
  log, root = life(sand)
  quoted = engine.rung("<S1>hi</S1>\nk = S1", on=root)
  assert await quoted is None
  with pytest.raises(Refused):
    await engine.rung("x = BAD", on=root)
  refused = said(log, "rung")[-1][1]
  word, got = engine.ask("program", root)
  assert word == ("program", f"program@{OPERATOR}.4", OPERATOR, root) and isinstance(got, dict)
  assert got == {quoted: "S1 = 'hi'\nk = S1"} and refused not in got
  assert [a for a in said(log, "done") if a[1] == word[1]] == [("done", word[1], root, got)]
  assert engine.modules[root]["k"] == "hi"


async def test_a_chain_with_a_source_asks_the_program_of_its_origin() -> None:
  """A chain with a source asks the program of its origin, and gate asks the program of the chain before it asks the gate whether the word may run."""
  sand: Sand = sown()
  _, root = life(sand)
  step = engine.rung("k = 1", on=root)
  assert await step is None
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.gate("j = k", on=root) == []
  programs = [one for one in engine.asked.values() if one[0] == "program"]
  assert [one[2:] for one in programs] == [(twin, root), (OPERATOR, root)]
  assert [engine.outcomes[one[1]] for one in programs] == [{step: "k = 1"}, {step: "k = 1"}]
  last = list(engine.asked.values())[-2:]
  assert last == [programs[-1], ("gate", last[1][1], OPERATOR, root, "j = k", {step: "k = 1"})]
