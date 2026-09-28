"""told, the paragraph that an act tells of itself."""

from conftest import born, paragraphs, said, settle
from furb import engine
from furb.engine import OPERATOR


async def test_the_open_of_an_act_tells_the_id_and_what_the_act_says_of_itself() -> None:
  """The open of an act tells the id and what the act says of itself, and no actor and no arguments as such."""
  _, log, root = born()
  act = engine.bash("echo hi", fed=True, timeout=9.0, on=root)
  assert (await act).code == 0
  await settle()
  opened = next(a[3] for a in said(log, "tell") if a[1] == act)
  assert opened == [f"#{act}\n{act}_command = 'echo hi'", f"{act}: Act[Exit] = Act('{act}')"]
  assert said(log, "bash")[0][4:] == ("echo hi", True, 9.0)


async def test_a_closed_paragraph_binds_what_the_act_came_to_under_the_word_value() -> None:
  """A closed paragraph binds what the act came to under the word value."""
  _, _, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  engine.close(21, act)
  assert await act == 21
  other = engine.prompt(str, "which?", to=OPERATOR, on=root)
  engine.close("k", other)
  assert await other == "k"
  lines = engine.prompt(str, "which lines?", to=OPERATOR, on=root)
  engine.close("one\ntwo", lines)
  assert await lines == "one\ntwo"
  step = engine.rung("k = 1\nclose(5)", on=root)
  assert await step == 5
  await settle()
  closed = [one for one in paragraphs(engine.turns(on=root)) if one.split("\n")[0].endswith(" closed")]
  assert closed == [
    f"#{act} closed\n{act}_value = 21",
    f"#{other} closed\n{other}_value = 'k'",
    f"#{lines} closed\n<s:{lines}_value>\none\ntwo</s:{lines}_value>",
    f"#{step} closed\n{step}_value = 5",
  ]


async def test_told_gives_the_saying_of_a_tell_about_an_act() -> None:
  """told gives the saying of a tell about an act, with one paragraph headed with the id of the act, which an ear yields and a verb says, so a chain holds what it told where it told it."""
  _, log, root = born()
  act = engine.rung("k = 1", on=root)
  assert await act is None
  told = [a for a in said(log, "tell") if a[1] == act]
  assert [a[3] for a in told] == [[f"#{act}\n{act}_word = 'k = 1'"]]
  saying = engine.told(act, "said", "more = 1", count=2)
  assert saying == ("tell", act, [f"#{act} said\n{act}_count = 2", "more = 1"])
  made = engine.say(*saying)
  assert made == ("tell", act, OPERATOR, [f"#{act} said\n{act}_count = 2", "more = 1"])
  held = engine.transcript(root)
  assert [a for a in held if a[0] == "tell" and a[1] == act] == [*told, made]
