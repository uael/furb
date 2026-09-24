"""under, whether one act is another or was made by it."""

from conftest import STANDS, Sand, life, said
from furb import engine
from furb.engine import OPERATOR, WORLD


async def made(sand: Sand) -> tuple[str, str, str, str]:
  """A life whose prompt makes a rung, whose word starts a wait: the chain, the prompt, the rung and the wait."""
  log, root = life(sand)
  sand.script[root] = ["x = wait(0)\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  return root, said(log, "prompt")[0][1], said(log, "rung")[0][1], said(log, "wait")[0][1]


async def test_whether_one_act_is_another_or_was_made_by_it_which_the_life_says() -> None:
  """Whether one act is another or was made by it, which the life says, since every question says who made it."""
  _, asking, step, command = await made(Sand(stands=STANDS))
  assert engine.under(asking, asking)
  assert engine.under(step, asking) and engine.under(command, step)
  assert not engine.under(asking, step)
  assert (engine.acts[step][2], engine.acts[command][2]) == (asking, step)


async def test_an_act_is_under_every_ancestor_of_the_act() -> None:
  """An act is under every ancestor of the act, which the maker of each says in turn, up to the operator or an ear of the outside."""
  root, asking, step, command = await made(Sand(stands=STANDS))
  assert (asking, step, command) == ("prompt1", "rung1", "wait1")
  assert engine.under(command, step) and engine.under(command, asking) and engine.under(command, command)
  assert engine.acts[asking][2] == OPERATOR and engine.under(command, OPERATOR)
  token = engine.site.set(WORLD)
  try:
    reading, _ = engine.ask("clock", root)
  finally:
    engine.site.reset(token)
  assert reading[2] == WORLD and engine.under(reading[1], WORLD) and not engine.under(reading[1], OPERATOR)


async def test_an_act_is_under_its_chain_only_when_the_chain_made_it() -> None:
  """An act is under its chain only when the chain made it: the rung of a prompt the operator made is under that prompt, and on the chain, so the maker of an act says who made it and never where it stands."""
  where, asking, step, _ = await made(Sand(stands=STANDS))
  assert engine.under(step, asking) and not engine.under(step, where)
  _, held = engine.ask("transcript", where, where)
  assert isinstance(held, list)
  assert [a[3] for a in held if a[0] == "rung" and a[1] == step] == [where]


async def test_nothing_is_under_a_name_of_nothing() -> None:
  """Nothing is under a name of nothing."""
  _, _, _, command = await made(Sand(stands=STANDS))
  assert not engine.under(command, "")
  assert not engine.under("wait9", "wait9x")
  assert not engine.under("", "")
  assert not engine.under("", "chain1")
