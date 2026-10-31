"""accepts, the policy of the codes of the commands of a chain."""

from conftest import born, said, settle
from furb import engine


async def test_accepts_is_the_policy_of_the_codes_of_the_commands_of_a_chain_that_give_no_ok() -> None:
  """accepts is the policy of the codes of the commands of a chain that give no ok: a function of the command and its Exit that says whether a word that awaits the command accepts it, and None in the module of the engine, where the default policy of bash rules."""
  sand, log, root = born(
    "accepts = lambda command, exit: exit.stdout.content == 'fine\\n'\n"
    "x = bash('one')\ny = bash('two')\ngot = [(await x).code]\ntry:\n  await y\nexcept Refused:\n  got.append('refused')\nclose(got)",
    auto=False,
  )
  assert engine.accepts is None and engine.module(root)["accepts"] is None
  asked = engine.prompt(list, "run them", on=root)
  await settle()
  one, two = (a[1] for a in said(log, "bash"))
  sand.writes(one, "fine\n", "stdout")
  sand.exits(one, 7)
  sand.exits(two, 0)
  assert await asked == [7, "refused"]


async def test_a_word_binds_accepts_like_any_name() -> None:
  """A word binds accepts like any name, so the policy rules each later command with no ok on its chain, and the last binding wins."""
  sand, log, root = born(
    "accepts = lambda command, exit: exit.code == 3\n"
    "x = bash('a')\ngot = [(await x).code]\n"
    "accepts = lambda command, exit: exit.code == 4\n"
    "y = bash('b')\ngot.append((await y).code)\nclose(got)",
    auto=False,
  )
  asked = engine.prompt(list, "run them", on=root)
  await settle()
  sand.exits(said(log, "bash")[0][1], 3)
  await settle()
  sand.exits(said(log, "bash")[1][1], 4)
  assert await asked == [3, 4]
