"""Program, the question of the words of the rungs of a chain that the gate let run."""

import pytest

from conftest import WORD, Sand, lived, said, settle, sown
from furb import engine
from furb.engine import OPERATOR, Refused


async def test_a_program_is_the_question_of_the_words_of_the_rungs_of_a_chain_that_the_gate_let_run() -> None:
  """A program is the question of the words of the rungs of a chain that the gate let run, each with the name of its rung, which the chain answers."""
  sand: Sand = sown()
  log, root = await lived(sand)
  with pytest.raises(Refused):
    await engine.rung("x = BAD", on=root)
  steps = [one[1] for one in said(log, "rung")]
  word, got = engine.ask("program", root)
  match word:
    case ("program", _, by, on):
      asked = (by, on)
    case _:
      asked = None
  assert asked == (OPERATOR, root)
  assert got == {steps[0]: WORD, steps[1]: "close(None)"}


async def test_a_chain_with_a_source_asks_the_program_of_its_origin() -> None:
  """A chain with a source asks the program of its origin, and gate asks the program of the chain before it asks the Kernel."""
  sand: Sand = sown()
  _, root = await lived(sand)
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.gate("k = 9", on=root) == []
  asked = [(one[0], one[2], one[3], *one[4:]) for one in engine.asked.values() if one[0] == "program"]
  assert asked == [("program", twin, root), ("program", OPERATOR, root)]
  gates = []
  for one in engine.asked.values():
    match one:
      case ("gate", _, by, on, word, dict(program)):
        gates.append((by, on, word, list(program.values())))
  assert gates[-1] == (OPERATOR, root, "k = 9", [WORD, "close(None)"])
