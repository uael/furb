"""told, the tag that an act tells of itself."""

import pytest

from conftest import STANDS, Sand, attr, life, said, settle, tags
from furb import engine
from furb.engine import OPERATOR


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_the_opened_tag_tells_the_id_and_what_the_act_says_of_itself() -> None:
  """The opened tag tells the id and what the act says of itself, and no actor and no arguments as such."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", on=root)
  assert (await act).code == 0
  await settle()
  _, command, *_ = said(log, "bash")[0]
  opened = [tag for tag in tags(engine.turns(on=root), "opened") if attr(tag, "id") == command]
  assert opened == [("opened", [("id", command), ("command", "echo hi")], None)]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_a_closed_tag_tells_the_act_with_what_it_came_to_as_python_shows_it() -> None:
  """A closed tag tells the act with what it came to, as python shows it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  engine.close(21, act)
  assert await act == 21
  other = engine.prompt(str, "which?", to=OPERATOR, on=root)
  engine.close({"k": 1}, other)
  assert await other == {"k": 1}
  command = engine.bash("echo hi", on=root)
  assert (await command).code == 0
  await settle()
  closed = tags(engine.turns(on=root), "closed")
  assert [(attr(tag, "over"), tag[2]) for tag in closed if ("over", act) in tag[1]] == [(act, "21")]
  assert [(attr(tag, "over"), tag[2]) for tag in closed if ("over", other) in tag[1]] == [(other, "{'k': 1}")]
  assert [attr(tag, "code") for tag in closed if ("id", said(log, "bash")[0][1]) in tag[1]] == [0]


@pytest.mark.xfail(strict=True, raises=NotImplementedError)
async def test_told_says_a_tell_about_an_act_with_one_tag_that_names_the_act_by_its_id() -> None:
  """told says a tell about an act with one tag that names the act by its id, and gives the tell back, so a chain holds what it told where it told it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 1", on=root)
  assert await act is None
  told = [a for a in said(log, "tell") if a[1] == act]
  assert [tag[1][0] for a in told for tag in a[3]] == [("id", act), ("id", act)]
  assert [len(a[3]) for a in told] == [1, 1]
  made = engine.told("opened", act, ("k", 1))
  assert made == ("tell", act, OPERATOR, [("opened", [("id", act), ("k", 1)], None)])
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a for a in held if a[0] == "tell" and a[1] == act] == [*told, made]
