"""Pause, the control that holds what it is over."""

from asyncio import CancelledError

from conftest import STANDS, Sand, heads, life, said, settle
from furb import engine
from furb.engine import OPERATOR, WORLD, Refused

COST = (80000, 0, 0, 0, 1.5)
"""One answer of a model: a dollar and a half, and a fifth of the window of the actor the suite stands on."""


async def test_while_a_pause_stands_nothing_that_the_pause_is_over_hears() -> None:
  """While a pause stands, nothing that the pause is over hears, and what is said meanwhile waits for the wake."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(root)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes
  assert [one for one in said(log, "done") if one[1] == command] == []
  engine.wake(root)
  await settle()
  assert (await act) == 0


async def test_a_control_is_a_fact_over_an_act() -> None:
  """A control is a fact over an act: over that act, over everything that act made, and over everything on a chain when the control names a chain."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  engine.cancel(act)
  await settle()
  assert [isinstance(engine.outcomes[one], CancelledError) for one in (act, step, command)] == [True, True, True]
  mine = engine.bash("elsewhere", on=root)
  engine.cancel(root)
  await settle()
  assert isinstance(engine.outcomes[mine], CancelledError)


async def test_a_control_reaches_what_it_is_over_and_whatever_else_its_words_name() -> None:
  """A control reaches what it is over, and whatever else the words of the control name."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  step = said(log, "rung")[0][1]
  engine.close(21, act)
  await settle()
  word = said(log, "close")[0]
  assert (word[1], word[3]) == (act, 21)
  assert engine.outcomes[act] == 21 and isinstance(engine.outcomes[step], CancelledError)


async def test_a_control_is_no_act_it_takes_no_name_of_its_own() -> None:
  """A control is no act: it takes no name of its own, and the record holds it as a fact about the acts it is over."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  act = engine.bash("slow", on=root)
  made = set(engine.acts)
  engine.pause(act)
  engine.cancel(act)
  await settle()
  assert set(engine.acts) == made
  assert [(one[0], one[1]) for one in log if one[0] in ("pause", "cancel")] == [("pause", act), ("cancel", act)]
  kept = [fact for fact, *_ in sand.record if fact[0] in ("pause", "cancel")]
  assert [(one[0], one[1], one[2]) for one in kept] == [("pause", act, OPERATOR), ("cancel", act, OPERATOR)]


async def test_what_a_control_reaches() -> None:
  """What a control reaches: the act it names, everything that act made, and every act of the chain it names."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  engine.cancel(act)
  over = said(log, "cancel")[0]
  assert [engine.covers(over, one) for one in (act, step, command)] == [True, True, True]
  engine.cancel(root)
  whole = said(log, "cancel")[-1]
  mine = engine.bash("elsewhere", on=root)
  assert engine.covers(whole, mine) and not engine.covers(over, mine)


async def test_it_reaches_by_the_chain_as_well_as_by_the_name() -> None:
  """It reaches by the chain as well as by the name, since an act on a chain is not under it unless the chain made it."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  act = engine.bash("slow", on=root)
  assert not engine.under(act, root)
  engine.cancel(root)
  await settle()
  assert isinstance(engine.outcomes[act], CancelledError)


async def test_a_control_carries_the_header_it_tells() -> None:
  """A control carries the header it tells, so a model reads what was done to its work whoever did it, and nothing else writes that header: the chain that pauses a chain at its ceiling, or closes a prompt it will not serve, says the control the one way there is to say it."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  closed = f"#{ghost} closed Refused('ghost no actor')"
  shut = said(log, "close")[0]
  assert shut[:3] == ("close", ghost, root) and shut[4] == [closed]
  assert isinstance(shut[3], Refused) and str(shut[3]) == "ghost no actor"
  ceiling = engine.grant(usd=1.0, on=root)
  sand.script[root] = ["a = 1", "close(2)"]
  engine.prompt(int, "count", on=root)
  await settle()
  assert said(log, "pause") == [("pause", root, ceiling, [f"#{root} paused"])]
  assert [line for line in heads(engine.turns(on=root)) if line in (closed, f"#{root} paused")] == [
    closed,
    f"#{root} paused",
  ]
  told = [(a[0], a[-1]) for a in log if a[0] in ("tell", "pause", "wake", "cancel", "close")]
  assert [kind for kind, notes in told if closed in notes or f"#{root} paused" in notes] == ["close", "pause"]


async def test_a_pause_is_over_the_act_it_names_and_everything_under_it() -> None:
  """A pause is over the act it names and everything under it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  engine.pause(act)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert act not in engine.outcomes and command not in engine.outcomes
  engine.wake(act)
  await settle()
  assert (await act) == 0
