"""Filter, what says which acts of a transcript the turns of a chain with a source keep."""

import pytest

from conftest import STANDS, Sand, life, lived, plain, relived, said, seen, settle, sown, tags
from furb import engine


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_filter_is_given_the_acts_of_the_transcript_up_to_the_source() -> None:
  """A filter is given the acts of the transcript up to the source of a chain that has one."""
  sand = sown()
  log, root = await lived(sand)
  held: list[tuple] = []
  engine.chain("twin", source=root, filter=seen(held))
  await settle()
  _, theirs = engine.ask("transcript", root, root)
  assert isinstance(theirs, list)
  facts = [one[1] for one in theirs if engine.question(one)]
  assert [one[1] for one in held] == facts[: len(held)]
  named = {said(log, "prompt")[0][1], said(log, "rung")[0][1], said(log, "bash")[0][1]}
  assert named <= {one[1] for one in held}


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_filter_says_which_acts_the_turns_of_that_chain_keep_each_with_its_entries() -> None:
  """A filter says which acts the turns of that chain keep, each with its entries."""
  sand = sown()
  log, root = await lived(sand)
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  narrow = engine.chain("narrow", source=root, filter=engine.take(command, inside=False))
  await settle()
  told = tags(engine.turns(on=narrow))
  assert [tag for tag in told if ("id", command) in tag[1]] == []
  assert [tag for tag in tags(engine.turns(on=root)) if ("id", command) in tag[1]] != []
  assert [tag[0] for tag in told if ("id", step) in tag[1]] == ["opened"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_filter_is_any_callable_of_that_shape() -> None:
  """A filter is any callable of that shape, so a word adds a filter by writing one, and take makes the filter of the file."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = [
    "x = bash('echo one')\nn = (await x).code",
    "def bare(facts):\n  return [a for a in facts if not under(a[1], x)]\n\nclose(chain('side', source=__name__, filter=bare))",
  ]
  side = await engine.prompt(str, "fork", on=root)
  await settle(200)
  command = said(log, "bash")[0][1]
  assert [tag for tag in tags(engine.turns(on=side)) if ("id", command) in tag[1]] == []
  assert engine.modules[side]["n"] == 0
  assert engine.take(command, inside=False)([engine.get(command)]) == []


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_later_life_runs_the_filter_again_and_keeps_the_same_acts() -> None:
  """A later life runs the filter again and keeps the same acts, since the word that opened the chain opens it again with it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = [
    "x = bash('echo hi')\nn = (await x).code",
    "close(chain('side', source=__name__, filter=take(x, inside=False)))",
  ]
  side = await engine.prompt(str, "fork", on=root)
  await settle(200)
  command = said(log, "bash")[0][1]
  was = tags(engine.turns(on=side))
  assert [tag for tag in was if ("id", command) in tag[1]] == []
  _, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert over == root and tags(engine.turns(on=side)) == was


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_filter_is_no_word_of_a_chain() -> None:
  """A filter is no word of a chain: the verb keeps it for what the chain holds, so no record holds one."""
  sand = sown()
  log, root = await lived(sand)
  held: list[tuple] = []
  side = engine.chain("side", source=root, filter=seen(held))
  await settle()
  kept = [fact for _, fact, *_ in sand.record if fact[0] == "chain"]
  assert [(one[1], one[4], one[5]) for one in kept] == [(root, "root", ""), (side, "side", root)]
  assert [len(one) for one in said(log, "chain")] == [6, 6] and held
