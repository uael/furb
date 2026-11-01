"""crafted, the rungs of words that their callers wrote, open on a chain."""

from conftest import born, settle
from furb import engine


async def test_the_rungs_with_a_word_and_an_actor_that_are_open_on_the_chain_of_an_act() -> None:
  """The rungs with a word and an actor that are open on the chain of an act, by their names, in the order that they were made, which a rung with a word, a prompt to a model and the open of the root wait for."""
  _, _, root = born("close(1)")
  assert await engine.prompt(int, "go", on=root) == 1
  first = engine.rung("await wait(60)", to="m/low", on=root)
  plain = engine.rung("await wait(60)", on=root)
  second = engine.rung("k = 2", to="m/low", on=root)
  await settle()
  assert (engine.crafted(root), engine.crafted(plain)) == ([first, second], [first, second])
  engine.cancel(first)
  await settle()
  assert engine.crafted(root) == []
  assert engine.module(root)["k"] == 2
  engine.cancel(plain)
