"""told, the paragraph that an act tells of itself."""

from conftest import STANDS, Sand, heads, life, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_the_open_of_an_act_tells_the_id_and_what_the_act_says_of_itself() -> None:
  """The open of an act tells the id and what the act says of itself, and no actor and no arguments as such."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.bash("echo hi", fed=True, timeout=9.0, on=root)
  assert (await act).code == 0
  await settle()
  opened = next(a[3] for a in said(log, "tell") if a[1] == act)
  assert opened == [f"#{act} echo hi", f"{act}: Act[Exit] = Act('{act}')"]
  assert said(log, "bash")[0][4:] == ("echo hi", True, 9.0)


async def test_a_closed_header_tells_the_act_with_what_it_came_to_as_python_shows_it() -> None:
  """A closed header tells the act with what it came to, as python shows it."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  engine.close(21, act)
  assert await act == 21
  other = engine.prompt(str, "which?", to=OPERATOR, on=root)
  engine.close("k", other)
  assert await other == "k"
  step = engine.rung("k = 1\nclose(5)", on=root)
  assert await step == 5
  await settle()
  closed = [head for head in heads(engine.turns(on=root)) if " closed" in head]
  assert closed == [f"#{act} closed 21", f"#{other} closed 'k'", f"#{step} closed 5"]


async def test_told_says_a_tell_about_an_act_with_one_paragraph_headed_with_the_id_of_the_act() -> None:
  """told says a tell about an act with one paragraph headed with the id of the act, and gives the tell back, so a chain holds what it told where it told it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 1", on=root)
  assert await act is None
  told = [a for a in said(log, "tell") if a[1] == act]
  assert [a[3] for a in told] == [[f"#{act}", "k = 1"]]
  made = engine.told(act, "said", "# more")
  assert made == ("tell", act, OPERATOR, [f"#{act} said", "# more"])
  _, held = engine.ask("transcript", root, root)
  assert isinstance(held, list)
  assert [a for a in held if a[0] == "tell" and a[1] == act] == [*told, made]
