"""Actor, an actor the World offers."""

from conftest import born, paragraphs, said, settle
from furb import engine
from furb.engine import Refused


async def test_an_actor_the_world_offers() -> None:
  """An actor the World offers: the name of a model, the efforts it takes, and the window it reads."""
  _, _, root = born()
  assert paragraphs(engine.turns(on=root)) == [
    "#chain1\nchain1_label = 'root'\nchain1: Act[object] = Act('chain1')",
    "#chain1 standing\nchain1_roster = [['operator', [], 200000], ['m', ['low', 'high'], 400000], ['n', ['low'], 200000]]\n"
    "chain1_cwd = '/w'\n"
    "chain1_actor = 'm/low'",
  ]


async def test_the_window_that_a_roster_entry_leaves_unsaid_is_the_window_that_the_file_names() -> None:
  """The window that a roster entry leaves unsaid is the window that the file names."""
  assert engine.WINDOW == 200000
  ledgers = []
  for roster in ([["plain", [], 0]], [["plain", [], 400000]]):
    _, _, root = born("close(1)", stands=[roster, "/w", "plain"], cost=(100000, 0, 0, 0, 0.0))
    engine.grant(share=0.9, on=root)
    assert await engine.thread(int, "count", on=root) == 1
    await settle()
    ledgers += [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#grant1 ledger")]
  assert ledgers == [
    f"#grant1 ledger\ngrant1_spent = 0.0\ngrant1_filled = {100000 / engine.WINDOW}",
    f"#grant1 ledger\ngrant1_spent = 0.0\ngrant1_filled = {100000 / 400000}",
  ]


async def test_what_a_thread_names_is_one_of_these_names_and_one_effort_of_that_range() -> None:
  """What a thread names is one of these names and one effort of that range."""
  _, log, root = born()
  right = engine.thread(int, "hi", to="m/high", on=root)
  await settle()
  assert engine.peek(right) is None and [a[4] for a in said(log, "reply")] == ["m/high"]
  ghost = engine.thread(int, "hi", to="ghost/low", on=root)
  engine.cancel(right)
  await settle()
  assert isinstance(engine.peek(ghost), Refused)
  assert [a[4] for a in said(log, "reply")] == ["m/high"]


async def test_an_actor_takes_an_effort_of_its_own_and_any_actor_takes_the_effort_that_is_not_named() -> None:
  """An actor takes an effort of its own, and any actor takes the effort that is not named."""
  sand, log, root = born()
  for to in ("m/low", "m/high", "m", "n"):
    sand.script[root] = ["close(1)"]
    assert await engine.thread(int, "hi", to=to, on=root) == 1
  assert [a[4] for a in said(log, "reply")] == ["m/low", "m/high", "m", "n"]
  wrong = engine.thread(int, "hi", to="n/high", on=root)
  await settle()
  assert isinstance(engine.peek(wrong), Refused)
