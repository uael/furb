"""Failed, what a word that awaits a command raises when the command fails."""

from conftest import born, said, settle
from furb import engine
from furb.engine import Exit, Text


async def test_what_a_word_that_awaits_a_command_raises_when_the_command_exits_with_a_code_that_it_does_not_accept() -> (
  None
):
  """What a word that awaits a command raises when the command exits with a code that it does not accept, so the word stops where the command failed and tells why."""
  sand, log, root = born(
    "x = bash('false')\ntry:\n  await x\nexcept Failed as no:\n  close(type(no).__name__)", auto=False
  )
  asked = engine.prompt(str, "run it", on=root)
  await settle()
  sand.exits(said(log, "bash")[0][1], 2)
  assert await asked == "Failed"


async def test_its_message_names_the_command_and_how_it_ended() -> None:
  """Its message names the command and how it ended, as bash3 exited 1, or bash3 timed out after 5.0s."""
  sand, log, root = born(
    "one = bash('false')\ntwo = bash('slow', timeout=5.0)\nsaid = []\nfor x in (one, two):\n  try:\n    await x\n  except Failed as no:\n    said.append(str(no))\nclose(said)",
    auto=False,
  )
  asked = engine.prompt(list, "run them", on=root)
  await settle()
  one, two = (a[1] for a in said(log, "bash"))
  sand.exits(one, 1)
  sand.exits(two, None)
  assert await asked == [f"{one} exited 1", f"{two} timed out after 5.0s"]


async def test_a_word_that_catches_it_reads_the_exit_of_the_command_with_peek() -> None:
  """A word that catches it reads the Exit of the command with peek, as peek(bash3)."""
  sand, log, root = born(
    "x = bash('false')\ntry:\n  await x\nexcept Failed:\n  got = peek(x)\n  assert isinstance(got, Exit)\n  close(got.code)",
    auto=False,
  )
  asked = engine.prompt(int, "run it", on=root)
  await settle()
  one = said(log, "bash")[0][1]
  sand.exits(one, 3)
  assert await asked == 3
  assert engine.peek(one) == Exit(3, Text(f"{one}/stdout"), Text(f"{one}/stderr"))
