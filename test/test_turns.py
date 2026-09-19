"""turns, what a model reads of a chain."""

import pytest

from conftest import STANDS, Sand, life, said, settle, shown, tags
from furb import engine
from furb.engine import span

CARRY = ("tell", "pause", "wake", "cancel", "close")
"""The kinds of fact that carry tags, of which the turns are folded."""


def carrying(held: list[tuple]) -> list[tuple]:
  """Every tag that the facts of a transcript carry, in the order they were said."""
  return [tag for a in held if a[0] in CARRY for tag in (a[4] if a[0] == "close" else a[3])]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_turns_of_a_chain_folded_from_what_it_has_heard() -> None:
  """The turns of a chain, folded from what it has heard."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  got = engine.turns(on=root)
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert got == engine.turns_of(held)
  assert [role for role, *_ in got] == ["user"]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_turns_asked_from_a_run_tells_how_many_turns_there_are() -> None:
  """A turns asked from a run tells how many turns there are, and never the turns."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(len(turns()))"]
  assert await engine.prompt(int, "count them", on=root) == 3
  assert tags(engine.turns(on=root), "turns") == [("turns", [("turns", 3)], None)]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_turns_of_what_a_chain_has_heard() -> None:
  """The turns of what a chain has heard: every fact that carries tags stands as its tags, and nothing else stands at all."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await settle()
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  carried = carrying(held)
  assert tags(engine.turns(on=root)) == carried
  assert carried != [] and [a[0] for a in held if a[0] not in CARRY] != []


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_turn_a_model_was_answered_with_closes_the_turn_of_the_operator() -> None:
  """The turn a model was answered with closes the turn of the operator and stands as the turn it is, and a text stands by the lines it has not seen, which the one that tells it says the show of."""
  sand = Sand(files={"/w/n.txt": "one\ntwo\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["read('n.txt', span(2, 2))\nclose(1)"]
  assert await engine.prompt(int, "read it", on=root) == 1
  got = engine.turns(on=root)
  assert [role for role, *_ in got] == ["user", "assistant", "user"]
  assert got[1] == said(log, "answer")[0][3]
  assert shown(tags(got, "read")[0]) == [("shown", [("path", "/w/n.txt"), ("known", 0)], "2 two")]
  assert span(2, 2)(["one", "two"]) == [2]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_chain_answers_for_its_turns() -> None:
  """The chain answers for its turns, folded from what it has heard."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("cd('/x')", on=root)
  got = engine.turns(on=root)
  assert [role for role, *_ in got] == ["user"]
  assert [name for name, *_ in tags(got)] == ["opened", "opened", "opened", "cd", "closed"]
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  asking = said(held, "turns")[-1]
  assert [a[2] for a in said(held, "done") if a[1] == asking[1]] == [root]
