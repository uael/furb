"""Refused, what a call the engine will not make raises."""

import pytest

from conftest import Dead, born, life, said, settle
from furb import engine
from furb.engine import Exit, Refused, Text


async def test_what_a_call_the_engine_will_not_make_raises_in_the_one_that_made_it() -> None:
  """What a call the engine will not make raises in the one that made it."""
  born()
  with pytest.raises(Refused, match="no chain"):
    engine.bash("nowhere")
  with pytest.raises(Refused, match="no chain"):
    engine.chain("twin", source="chain://operator.9")


async def test_a_refused_call_raises_refused_in_the_caller() -> None:
  """A refused call raises Refused in the caller."""
  _, _, root = born()
  act = engine.rung("k = BAD", on=root)
  with pytest.raises(Refused):
    await act
  assert isinstance(engine.peek(act), Refused)
  _, other = life(Dead())
  word = "try:\n  read('a.txt')\nexcept Refused as no:\n  close(str(no))"
  assert await engine.rung(word, on=other) == "a dead World answers no read"


async def test_a_word_that_awaits_a_command_whose_code_the_command_does_not_accept_is_refused_where_it_waited() -> None:
  """A word that awaits a command whose code the command does not accept is refused where it waited, with a message that names the command and how it ended, as bash3 exited 1, or bash3 timed out after 5.0s."""
  sand, log, root = born(
    "one = bash('false')\ntwo = bash('slow', timeout=5.0)\nsaid = []\nfor x in (one, two):\n  try:\n    await x\n  except Refused as no:\n    said.append(str(no))\nclose(said)",
    auto=False,
  )
  asked = engine.prompt(list, "run them", on=root)
  await settle()
  one, two = (a[1] for a in said(log, "bash"))
  sand.exits(one, 1)
  sand.exits(two, None)
  assert await asked == [f"{one} exited 1", f"{two} timed out after 5.0s"]


async def test_a_word_that_catches_that_refusal_reads_the_exit_of_the_command_with_peek() -> None:
  """A word that catches that refusal reads the Exit of the command with peek, as peek(bash3)."""
  sand, log, root = born(
    "x = bash('false')\ntry:\n  await x\nexcept Refused:\n  got = peek(x)\n  assert isinstance(got, Exit)\n  close(got.code)",
    auto=False,
  )
  asked = engine.prompt(int, "run it", on=root)
  await settle()
  one = said(log, "bash")[0][1]
  sand.exits(one, 3)
  assert await asked == 3
  assert engine.peek(one) == Exit(3, Text(f"{one}/stdout"), Text(f"{one}/stderr"))
