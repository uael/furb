"""under, whether one act is another or was made by it."""

import pytest

from conftest import STANDS, Sand, life, said
from furb import engine


async def made(sand: Sand) -> tuple[str, str, str, str]:
  """A life whose prompt makes a rung, whose word starts a command: the chain, the prompt, the rung and the command."""
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  return root, said(log, "prompt")[0][1], said(log, "rung")[0][1], said(log, "bash")[0][1]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_whether_one_act_is_another_or_was_made_by_it_which_their_lineages_say() -> None:
  """Whether one act is another or was made by it, which their lineages say, since an act is named under the one that made it."""
  _, asking, step, command = await made(Sand(stands=STANDS))
  assert engine.under(asking, asking)
  assert engine.under(step, asking) and engine.under(command, step)
  assert not engine.under(asking, step)
  assert engine.lineage(step) == engine.lineage(asking) + ".1"


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_an_act_is_under_every_ancestor_of_the_act() -> None:
  """An act is under every ancestor of the act, and the name of the act says which acts those are."""
  _, asking, step, command = await made(Sand(stands=STANDS))
  assert (asking, step, command) == ("prompt://operator.2", "rung://operator.2.1", "bash://operator.2.1.1")
  assert engine.under(command, step) and engine.under(command, asking) and engine.under(command, command)


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_an_act_is_under_its_chain_only_when_the_chain_made_it() -> None:
  """An act is under its chain only when the chain made it: the rung of a prompt the operator made is under that prompt, and on the chain, so a name says who made an act and never where it stands."""
  where, asking, step, _ = await made(Sand(stands=STANDS))
  assert engine.under(step, asking) and not engine.under(step, where)
  _, held = engine.ask("transcript", where, where)
  assert isinstance(held, list)
  assert [a[3] for a in held if a[0] == "rung" and a[1] == step] == [where]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_nothing_is_under_a_name_of_nothing() -> None:
  """Nothing is under a name of nothing."""
  assert not engine.under("bash://operator.2", "")
  assert not engine.under("", "")
  assert not engine.under("", "chain://operator.1")
