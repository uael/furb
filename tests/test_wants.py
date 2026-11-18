"""Wants, the act a run waits for."""

from conftest import born, dones, said, settle
from furb import engine


async def test_a_wants_is_the_act_that_a_run_makes_when_its_word_waits() -> None:
  """A wants is the act that a run makes when its word, or a task of its word, waits for an act: it takes itself as it is made, a run holds many at once, and the Kernel says each done, as that wants, with what its act came to once that act is done and no pause covers the rung, so a pause over the rung holds the word."""
  sand, log, root = born(auto=False)
  act = engine.rung("out = await bash('slow')\nclose(out.code)", on=root)
  await settle()
  _, command, *_ = said(log, "bash")[0]
  (run,) = [a[1] for a in said(log, "run") if a[4] == act]
  (wants,) = said(log, "wants")
  assert (wants[2], wants[3], wants[4]) == (run, root, command)
  assert [a[2] for a in said(log, "started") if a[1] == wants[1]] == [wants[1]]
  engine.pause(act)
  sand.exits(command, 0)
  await settle()
  assert dones(log, "wants") == [] and engine.peek(act, ...) is ...
  engine.wake(act)
  await settle()
  assert [(a[2], a[3]) for a in dones(log, "wants")] == [(wants[1], engine.peek(command))] and (await act) == 0
  sand, log, root = born(auto=False)
  word = "import asyncio\nasync def one(c):\n  return (await bash(c)).code\nclose(sum(await asyncio.gather(one('a'), one('b'))))"
  both = engine.rung(word, on=root)
  await settle()
  commands = [a[1] for a in said(log, "bash")]
  assert [a[4] for a in said(log, "wants")] == commands and len(commands) == 2
  assert [a[2] for a in said(log, "started") if a[1] in (made := [w[1] for w in said(log, "wants")])] == made
  engine.pause(both)
  sand.exits(commands[1], 0)
  sand.exits(commands[0], 0)
  await settle()
  assert dones(log, "wants") == []
  engine.wake(both)
  await settle()
  assert [a[1] for a in dones(log, "wants")] == [w[1] for w in said(log, "wants")] and (await both) == 0


async def test_a_word_that_awaits_an_act_that_is_over_goes_on_at_a_later_turn_of_the_bus() -> None:
  """A word that awaits an act that is over goes on at a later turn of the bus, through a wants, as when it awaits one that is not over, so it never holds the bus and the journal keeps each fact in its order."""
  sand, log, root = born(auto=False)
  one = engine.bash("slow", on=root)
  other = engine.chain("watch", on=root)
  word = f"n = 0\nwhile n < 5 and over({one!r}):\n  await wait(120, {one!r})\n  t = clock()\n  n += 1\n"
  sand.exits(one, 0)
  await settle()
  before = len(log)
  engine.rung(word, on=other)
  await settle()
  facts = log[before:]
  waits = [i for i, a in enumerate(facts) if a[0] == "wait"]
  kept = [i for i, a in enumerate(facts) if a[0] == "keep" and a[3][0][0] == "wait"]
  assert engine.module(other)["n"] == 5 and len(waits) == 5
  assert kept[0] < waits[1]
