"""automatic, the actor of automatic work."""

from collections.abc import Sequence
from pathlib import Path

from conftest import STANDS
from furb import engine
from skills.conftest import naming


async def places(at: Path, roster: Sequence[Sequence[object]]) -> list[str]:
  """What automatic gives at its first three places, in a life of its own in a new folder, on a roster."""
  at.mkdir()
  _, root = naming(at, roster, lives=False)
  got = await engine.rung("close([automatic(n) for n in range(3)] + [automatic()])", on=root)
  assert isinstance(got, list) and got[3] == got[0]
  return got[:3]


async def test_automatic_gives_the_actor_of_automatic_work(tmp_path: Path) -> None:
  """automatic gives the actor of automatic work: the name at place n of the list of automatic work, which holds the models of the roster of the standing that have a place in that list, which the entry of each model gives as its fifth field, in the order of their places, then each cheap model of the roster, each name once."""
  plain = [*STANDS[0], ["h", ["low"], 200000, "cheap"], ["s", [], 200000, "dear"], ["k", [], 200000, "cheap"]]
  placed = [*STANDS[0], ["h", ["low"], 200000, "cheap", 1], ["s", [], 200000, "dear", 0], ["k", [], 200000, "cheap"]]
  assert await places(tmp_path / "placed", placed) == ["s", "h", "k"]
  assert await places(tmp_path / "plain", plain) == ["h", "k", ""]


async def test_past_the_end_of_the_list_it_gives_an_empty_str(tmp_path: Path) -> None:
  """Past the end of the list it gives an empty str, so a use whose model refuses it asks automatic(1), then the next, until the list ends."""
  assert await places(tmp_path / "one", [*STANDS[0], ["h", [], 200000, "cheap"]]) == ["h", "", ""]
  assert await places(tmp_path / "none", [*STANDS[0], ["s", [], 200000, "dear"]]) == ["", "", ""]


async def test_the_operator_is_no_such_model(tmp_path: Path) -> None:
  """The operator is no such model."""
  roster = [[engine.OPERATOR, [], 200000, "cheap", 0], ["h", [], 200000, "dear", 1]]
  assert await places(tmp_path / "operated", roster) == ["h", "", ""]
