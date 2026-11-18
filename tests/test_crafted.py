"""crafted, the rungs of words that their callers wrote, open on a chain."""

from conftest import born, said, settle
from furb import engine


async def test_the_rungs_with_a_word_and_an_actor_that_are_open_on_the_chain_of_an_act() -> None:
  """The rungs with a word and an actor that are open on the chain of an act, by their names, in the order that they were made, which a rung with a word, a prompt to a model and the open of the root wait for."""
  _, _, root = born("close(1)")
  assert await engine.prompt(int, "go", on=root) == 1
  first = engine.rung("await wait(60, __name__)", to="m/low", on=root)
  plain = engine.rung("await wait(60, __name__)", on=root)
  second = engine.rung("k = 2", to="m/low", on=root)
  await settle()
  assert (engine.crafted(root), engine.crafted(plain)) == ([first, second], [first, second])
  engine.cancel(first)
  await settle()
  assert engine.crafted(root) == []
  assert engine.module(root)["k"] == 2
  engine.cancel(plain)


async def test_the_run_of_a_crafted_rung_is_done_once_its_word_is_over() -> None:
  """The run of a crafted rung is done once its word is over, so a task that its word left running goes on and holds no other word and no prompt of its chain."""
  sand, log, root = born("close(1)", auto=False)
  word = (
    "import asyncio\n"
    "codes = []\n"
    "async def watch():\n"
    "  b = bash('slow')\n"
    "  codes.append((await b).code)\n"
    "  codes.append((await b).code)\n"
    "t = asyncio.create_task(watch())\n"
    "await bash('first')"
  )
  made = engine.rung(word, to="m/low", on=root)
  await settle()
  commands = {a[4]: a[1] for a in said(log, "bash")}
  first, slow = commands["first"], commands["slow"]
  sand.exits(first, 0)
  await settle()
  assert (engine.peek(made, ...), engine.crafted(root)) == (None, [])
  go = engine.prompt(int, "go", on=root)
  await settle()
  assert engine.peek(go, ...) == 1
  sand.exits(slow, 0)
  await settle()
  assert await engine.rung("close(codes)", on=root) == [0, 0]
